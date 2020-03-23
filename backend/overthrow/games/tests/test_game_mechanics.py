from hypothesis import given, assume
from hypothesis.extra.django import TestCase, from_model
from django.db.models import Sum

from overthrow.games.models import Game, Tile, Player, Movement
from overthrow.games import coords
from overthrow.games.tests import strategies
from overthrow.games.factories import PlayerFactory


class ExploitsTestCase(TestCase):
    """ Everything user can do is allowed by game rules """

    @given(from_model(Game))
    def test_army_conservation(self, game):
        """ Army count must be preserved during move, when there are no battles (single player) """
        print(game)
        assume(Player.objects.filter(tiles__game=game).count() <= 1)
        before = Tile.objects.filter(game=game).aggregate(Sum('army'))
        game._simulate_movements()
        after = Tile.objects.filter(game=game).aggregate(Sum('army'))
        self.assertEqual(before, after)


class PossibilitiesTestCase(TestCase):
    """ User can do everything that is allowed by game rules """

    @given(game=strategies.board(), source=strategies.coords(), target=strategies.coords())
    def test_everything_is_reachable(self, game, source, target):
        assume(source != target)
        source_tile = Tile.objects.filter(game=game, **coords.as_dict(source)).first()
        assume(source_tile is not None)
        target_tile = Tile.objects.filter(game=game, **coords.as_dict(target)).first()
        assume(target_tile is not None)

        source_tile.owner = PlayerFactory(game=game)
        source_tile.army = 1
        source_tile.save()
        movement = Movement.objects.create(source=source_tile, target=target_tile, amount=1)

        distance_before = coords.distance(movement.source.coords, movement.target.coords)

        # go one step and refresh ORM cache
        game._simulate_movements()
        movement = Movement.objects.filter(id=movement.id).first()

        if movement is None:
            # we reached target
            target_tile.refresh_from_db()
            self.assertEqual(target_tile.army, 1)

        else:
            # check that we are coming closer
            distance_after = coords.distance(movement.source.coords, movement.target.coords)
            self.assertEqual(distance_before, distance_after + 1)
