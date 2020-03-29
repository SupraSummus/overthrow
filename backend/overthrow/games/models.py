from pprint import pformat
import itertools

from django.conf import settings
from django.contrib.auth import get_user_model
from django.core.exceptions import ValidationError
from django.db import models, transaction
from django.db.models import Q, F
from django.utils import timezone
from django.utils.translation import gettext_lazy as _

from . import coords
from overthrow.utils import UUIDModel


class Game(UUIDModel):
    tick = models.PositiveIntegerField(default=0)

    @staticmethod
    @transaction.atomic
    def generate_hexagonal(radius):
        game = Game.objects.create()
        tiles = []
        for x, y in itertools.product(
            range(-radius, radius + 1),
            range(-radius, radius + 1),
        ):
            z = -x - y
            if abs(x) <= radius and abs(y) <= radius and abs(z) <= radius:
                tiles.append(Tile(
                    game=game,
                    x=x, y=y, z=z,
                ))
        Tile.objects.bulk_create(tiles)
        return game

    def create_player(self, user):
        player = Player.objects.create(game=self, user=user)
        player.grant_initial_tiles(tile_count=10, army=10)
        return player

    @transaction.atomic
    def simulate(self):
        from overthrow.games.game_simulator import GameSimulator

        # lock and retrieve needed objects
        tiles = list(Tile.objects.filter(game=self).select_for_update())
        movements = list(Movement.objects.filter(source__game=self).select_for_update())

        # simulate
        simulator = GameSimulator(tiles, movements)
        simulator.simulate()

        # save new state
        Tile.objects.bulk_update([
            simulator.tiles_by_id[tile_id]
            for tile_id in simulator.tiles_to_be_updated
        ], ['army', 'owner_id'])
        Movement.objects.filter(id__in=simulator.movements_to_be_deleted).delete()
        for m in simulator.movements_to_be_created:
            m.game = self
        Movement.objects.bulk_create(simulator.movements_to_be_created)
        Movement.objects.bulk_update(simulator.movements_to_be_updated, ['amount'])

    @transaction.atomic
    def simulate_till_now(self):
        # simulate steps until we catch up
        assert False, "not really implemented"
        # select all realtaed things and lock them untill the end of transaction
        game = self.__class__.objects.filter(id=self.id).select_for_update().get()
        while game.started_at + (game.tick + 1) * settings.TICK_DURATION < timezone.now():
            game.simulate()
            game.tick += 1
            game.save()

    def as_plain(self):
        return {
            'tiles': [t.as_plain() for t in self.tiles.all()],
            'players': [p.id for p in self.players.all()],
        }

    def __repr__(self):
        return pformat(self.as_plain())


class Player(UUIDModel):
    game = models.ForeignKey(
        Game,
        on_delete=models.CASCADE,
        related_name="players",
    )
    user = models.ForeignKey(
        get_user_model(),
        on_delete=models.CASCADE,
        related_name="+",
    )

    class Meta:
        constraints = [
            models.UniqueConstraint(fields=['game', 'user'], name='unique_player'),
        ]

    @transaction.atomic
    def grant_initial_tiles(self, tile_count, army):
        free_tiles = self.game.tiles.filter(owner=None).select_for_update()

        # select tile nearest to origin
        nearest_to_origin = sorted(
            free_tiles,
            key=lambda tile: coords.distance(tile.coords, (0, 0, 0)),
        )
        if not nearest_to_origin:
            return []
        initial = nearest_to_origin[0]

        # select tiles nearest to initial tile
        granted_tiles = sorted(
            free_tiles,
            key=lambda tile: coords.distance(tile.coords, initial.coords),
        )[:tile_count]

        # set ownership
        for tile in granted_tiles:
            tile.owner = self
            tile.army = army
        Tile.objects.bulk_update(granted_tiles, ['owner', 'army'])
        return granted_tiles


class Tile(UUIDModel):
    game = models.ForeignKey(
        Game,
        on_delete=models.CASCADE,
        related_name="tiles",
    )

    # coords for the hex in cubic coordinate system
    x = models.IntegerField()
    y = models.IntegerField()
    z = models.IntegerField()

    owner = models.ForeignKey(
        Player,
        on_delete=models.SET_NULL,
        related_name="tiles",
        null=True,
    )
    army = models.PositiveIntegerField(
        default=0,
    )

    class Meta:
        constraints = [
            models.UniqueConstraint(fields=['game', 'x', 'y'], name='unique_xy_coords'),
            models.UniqueConstraint(fields=['game', 'x', 'z'], name='unique_xz_coords'),
            models.UniqueConstraint(fields=['game', 'y', 'z'], name='unique_yz_coords'),
        ]

    def __str__(self):
        return f'{self.x}, {self.y}, {self.z}'

    @property
    def coords(self):
        return (self.x, self.y, self.z)

    def as_plain(self):
        return {
            'id': self.id,
            'coords': self.coords,
            'owner': self.owner_id,
            'army': self.army,
            'outgoing_movements': [m.as_plain() for m in self.outgoing_movements.all()],
        }


class Movement(UUIDModel):
    source = models.ForeignKey(
        Tile,
        on_delete=models.CASCADE,
        related_name='outgoing_movements',
    )
    target = models.ForeignKey(
        Tile,
        on_delete=models.CASCADE,
        related_name='incoming_movements'
    )
    amount = models.PositiveIntegerField()

    class Meta:
        constraints = [
            models.UniqueConstraint(fields=['source', 'target'], name='unique_path'),
            models.CheckConstraint(check=Q(amount__gte=1), name='nonempty_amount'),
            models.CheckConstraint(
                check=~Q(source=F('target')),
                name='nonempty_distance',
            ),
        ]

    def clean(self):
        super().clean()

        if self.source.game_id != self.target.game_id:
            raise ValidationError(_('Source and target have to be tiles in the same game.'))

    def as_plain(self):
        return {
            'target': self.target.coords,
            'amount': self.amount,
        }

    def __repr__(self):
        return pformat(self.as_plain())
