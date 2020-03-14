import itertools

from django.db import models, transaction
from django.contrib.auth import get_user_model

from overthrow.utils import UUIDModel


class Game(UUIDModel):
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
