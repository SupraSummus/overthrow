from collections import defaultdict

from hypothesis import given, assume, settings
from hypothesis.extra.django import TestCase
from django.db.models import Sum

from overthrow.games.models import Game, Tile, Movement
from overthrow.games import coords
from overthrow.games.tests import strategies


class ExploitsTestCase(TestCase):
    """ Everything user can do is allowed by game rules """

    @given(game=strategies.games())
    def test_no_armies_for_free(self, game):
        """ Army count cannot grow during fight phase """
        before = {
            r['owner_id']: r['army__sum'] for r in
            Tile.objects.filter(game=game).values('owner_id').annotate(Sum('army'))
        }
        Game.simulate(game)
        after = {
            r['owner_id']: r['army__sum'] for r in
            Tile.objects.filter(game=game).values('owner_id').annotate(Sum('army'))
        }
        for player_id in before:
            self.assertLessEqual(after[player_id], before[player_id])

    def test_no_teleportation(self):
        """ Armies may move at most one step at the time """
        pass  # TODO

    def test_no_double_attack(self):
        """ Armies can deal limited amount of damage each turn """
        pass  # TODO


class PossibilitiesTestCase(TestCase):
    """ User can do everything that is allowed by game rules """

    @given(game=strategies.games(
        unowned_tiles=False,
        min_player_count=1,
        max_player_count=1,
        min_army_count=60,
        max_movement_amount=10,
    ))
    def test_movement_moves_armies(self, game):
        tile = Tile.objects.get(game=game, x=0, y=0, z=0)
        expected_tile_army_after = tile.army
        movement_happening = False
        for movement in Movement.objects.filter(source__game=game).select_related('source', 'target'):
            if movement.source_id == tile.id:
                movement_happening = True
                expected_tile_army_after -= movement.amount
            next_tile_coords = coords.next_on_path(movement.source.coords, movement.target.coords)
            if next_tile_coords == tile.coords:
                movement_happening = True
                expected_tile_army_after += movement.amount
        assume(movement_happening)
        Game.simulate(game)
        tile.refresh_from_db()
        self.assertEqual(tile.army, expected_tile_army_after)


class FacilitiesTestCase(TestCase):
    """ Test behaviours which make controlling the game easier. """

    @settings(print_blob=True)
    @given(game=strategies.games(
        max_radius=2,
        unowned_tiles=False,
        min_player_count=1,
        max_player_count=1,
        min_army_count=18,
        max_army_count=18,
        max_movement_amount=3,
        max_movement_count=6,
    ))
    def test_movements_work_on_long_distances(self, game):
        """ Movements transfers armies aross many tiles and many turns """
        tile = Tile.objects.get(game=game, x=0, y=0, z=0)
        amount_by_target = defaultdict(int)
        for movement in Movement.objects.filter(source__game=game).select_related('source', 'target'):
            next_tile_coords = coords.next_on_path(movement.source.coords, movement.target.coords)
            if next_tile_coords == tile.coords and movement.target_id != tile.id:
                amount_by_target[movement.target_id] += movement.amount
        assume(amount_by_target)
        Game.simulate(game)
        for target_id, amount in amount_by_target.items():
            movement = Movement.objects.filter(
                source=tile,
                target_id=target_id,
            ).get()
            self.assertEqual(movement.amount, amount)

    @settings(print_blob=True)
    @given(game=strategies.games(
        unowned_tiles=False,
        min_player_count=1,
        max_player_count=1,
    ))
    def test_movements_are_deleted_only_when_armies_reach_destination(self, game):
        def get_travel_required():
            travel_required = 0
            for m in Movement.objects.filter(source__game=game).select_related('source', 'target'):
                travel_required += coords.distance(m.source.coords, m.target.coords) * m.amount
            return travel_required
        will_move = 0
        for m in Movement.objects.filter(source__game=game).values(
            'source__id',
            'source__army',
        ).annotate(Sum('amount')):
            will_move += min(m['source__army'], m['amount__sum'])
        travel_required_before = get_travel_required()
        Game.simulate(game)
        travel_required_after = get_travel_required()
        self.assertEqual(travel_required_before - will_move, travel_required_after)
