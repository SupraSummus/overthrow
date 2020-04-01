from collections import defaultdict
from unittest import mock
import math
import random

from django.db.models import Sum
from hypothesis import given, assume
from hypothesis.extra.django import TestCase
import hypothesis

from overthrow.games import coords
from overthrow.games.game_simulator import (
    zip_dicts,
    DEFENSE_TO_ATTACK_EFFICIENCY,
    ATTACK_TO_DEFENSE_EFFICIENCY,
    ATTACK_TO_ATTACK_EFFICIENCY,
)
from overthrow.games.models import Tile, Movement, Player
from overthrow.games.factories import CorporationFactory
from overthrow.games.tests import strategies


def get_armies_by_user_id(game):
    return {
        r["owner_id"]: r["army__sum"]
        for r in Tile.objects.filter(game=game).values("owner_id").annotate(Sum("army"))
    }


class OSUrandomPatcher:
    def setUp(self):
        self._os_urandom_patcher = mock.patch(
            "os.urandom",
            new=lambda size: bytes(random.getrandbits(8) for _ in range(size)),
        )
        self._os_urandom_patcher.start()
        super().setUp()

    def tearDown(self):
        super().tearDown()
        self._os_urandom_patcher.stop()


class ExploitsTestCase(TestCase):
    """ Everything user can do is allowed by game rules """

    @given(game=strategies.games())
    def test_no_armies_from_nothing(self, game):
        """ Army count cannot grow during fight phase """
        before = get_armies_by_user_id(game)
        game.simulate()
        after = get_armies_by_user_id(game)
        for player_id, (b, a) in zip_dicts(before, after, default=0):
            self.assertLessEqual(a, b)

    @given(game=strategies.games())
    def test_no_teleportation(self, game):
        """ Armies may move at most one step at the time """
        max_armies_by_coords = defaultdict(int)
        for tile in Tile.objects.filter(game=game):
            max_armies_by_coords[tile.coords] += tile.army
            for d in coords.neighbour_deltas:
                max_armies_by_coords[coords.sum(tile.coords, d)] += tile.army
        game.simulate()
        for tile in Tile.objects.filter(game=game):
            self.assertLessEqual(tile.army, max_armies_by_coords[tile.coords])

    @given(
        game=strategies.games(
            unowned_tiles=False, min_player_count=1, max_player_count=3,
        )
    )
    def test_no_double_attack(self, game):
        """ Armies can deal limited amount of damage each turn """
        before = get_armies_by_user_id(game)
        game.simulate()
        after = get_armies_by_user_id(game)
        for player_id, (b, a) in zip_dicts(before, after, default=0):
            max_damage = (sum(before.values()) - b) * max(
                DEFENSE_TO_ATTACK_EFFICIENCY,
                ATTACK_TO_DEFENSE_EFFICIENCY,
                ATTACK_TO_ATTACK_EFFICIENCY,
            )
            self.assertGreaterEqual(a, math.floor(b - max_damage))


class PossibilitiesTestCase(OSUrandomPatcher, TestCase):
    """ User can do everything that is allowed by game rules """

    @given(
        game=strategies.games(
            unowned_tiles=False,
            min_player_count=1,
            max_player_count=1,
            min_army_count=18,
            max_movement_amount=3,
        )
    )
    def test_movement_moves_armies(self, game):
        tile = Tile.objects.get(game=game, x=0, y=0, z=0)
        expected_tile_army_after = tile.army
        movement_happening = False
        for movement in Movement.objects.filter(source__game=game).select_related(
            "source", "target"
        ):
            if movement.source_id == tile.id:
                movement_happening = True
                expected_tile_army_after -= movement.amount
            next_tile_coords = coords.next_on_path(
                movement.source.coords, movement.target.coords
            )
            if next_tile_coords == tile.coords:
                movement_happening = True
                expected_tile_army_after += movement.amount
        assume(movement_happening)
        game.simulate()
        tile.refresh_from_db()
        self.assertEqual(tile.army, expected_tile_army_after)

    @given(
        game=strategies.games(
            max_radius=2, min_player_count=1, max_army_count=1, max_movement_count=0,
        ),
        data=hypothesis.strategies.data(),
    )
    def test_tiles_can_be_conquered(self, game, data):
        player = Player.objects.filter(game=game).first()
        assume(player)

        player_tiles = list(Tile.objects.filter(game=game, owner=player))
        assume(player_tiles)

        # create user movements
        minimal_winning_amount = 1 + math.ceil(1 / ATTACK_TO_DEFENSE_EFFICIENCY)
        movements = data.draw(
            strategies.movement_sets(
                source_tiles=player_tiles,
                target_tiles=list(Tile.objects.filter(game=game)),
                min_movement_amount=minimal_winning_amount,
                max_movement_amount=minimal_winning_amount,
            ),
            label="player's movements",
        )
        assume(movements)

        Tile.objects.filter(game=game, owner=player).update(
            army=minimal_winning_amount * len(movements),
        )
        game.simulate()

        for m in movements:
            next_tile_coords = coords.next_on_path(m.source.coords, m.target.coords)
            tile = Tile.objects.get(game=game, **coords.as_dict(next_tile_coords))
            self.assertEqual(
                tile.owner_id, player.id,
            )

    @given(
        game=strategies.games(
            max_radius=1,
            unowned_tiles=False,
            min_player_count=2,
            max_player_count=2,
            min_movement_count=1,
            max_movement_count=1,
        ),
        data=hypothesis.strategies.data(),
    )
    def test_movement_across_friendly_borders(self, game, data):
        movement = (
            Movement.objects.filter(source__game=game)
            .select_related("source", "target")
            .first()
        )
        next_tile = Tile.objects.filter(
            game=game,
            **coords.as_dict(
                coords.next_on_path(movement.source.coords, movement.target.coords,)
            )
        ).first()
        assume(movement.source.owner_id != next_tile.owner_id)
        amount = min(movement.source.army, movement.amount)
        assume(amount > 0)
        expected_army = amount + next_tile.army

        # create management structure
        boss, subordinate = tuple(game.players.all())
        boss.corporation = CorporationFactory()
        boss.save()
        subordinate.boss = boss
        subordinate.save()

        game.simulate()

        next_tile.refresh_from_db()
        self.assertEqual(next_tile.army, expected_army)


class FacilitiesTestCase(TestCase):
    """ Test behaviours which make controlling the game easier. """

    @given(
        game=strategies.games(
            max_radius=2,
            unowned_tiles=False,
            min_player_count=1,
            max_player_count=1,
            min_army_count=18,
            max_army_count=18,
            max_movement_amount=3,
        )
    )
    def test_movements_work_on_long_distances(self, game):
        """ Movements transfers armies aross many tiles and many turns """
        tile = Tile.objects.get(game=game, x=0, y=0, z=0)
        amount_by_target = defaultdict(int)
        for movement in Movement.objects.filter(source__game=game).select_related(
            "source", "target"
        ):
            next_tile_coords = coords.next_on_path(
                movement.source.coords, movement.target.coords
            )
            if next_tile_coords == tile.coords and movement.target_id != tile.id:
                amount_by_target[movement.target_id] += movement.amount
        assume(amount_by_target)
        game.simulate()
        for target_id, amount in amount_by_target.items():
            movement = Movement.objects.filter(source=tile, target_id=target_id,).get()
            self.assertEqual(movement.amount, amount)

    @given(
        game=strategies.games(
            unowned_tiles=False, min_player_count=1, max_player_count=1,
        )
    )
    def test_movements_are_deleted_only_when_armies_reach_destination(self, game):
        def get_travel_required():
            travel_required = 0
            for m in Movement.objects.filter(source__game=game).select_related(
                "source", "target"
            ):
                travel_required += (
                    coords.distance(m.source.coords, m.target.coords) * m.amount
                )
            return travel_required

        will_move = 0
        for m in (
            Movement.objects.filter(source__game=game)
            .values("source__id", "source__army",)
            .annotate(Sum("amount"))
        ):
            will_move += min(m["source__army"], m["amount__sum"])
        travel_required_before = get_travel_required()
        game.simulate()
        travel_required_after = get_travel_required()
        self.assertEqual(travel_required_before - will_move, travel_required_after)
