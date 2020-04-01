import factory

from overthrow.users.factories import UserFactory
from . import models


class GameFactory(factory.django.DjangoModelFactory):
    class Meta:
        model = models.Game


class CorporationFactory(factory.django.DjangoModelFactory):
    class Meta:
        model = models.Corporation


class PlayerFactory(factory.django.DjangoModelFactory):
    class Meta:
        model = models.Player

    user = factory.SubFactory(UserFactory)


class TileFactory(factory.django.DjangoModelFactory):
    class Meta:
        model = models.Tile

    game = factory.SubFactory(GameFactory)


class MovementFactory(factory.django.DjangoModelFactory):
    class Meta:
        model = models.Movement

    source = factory.SubFactory(TileFactory)
    target = factory.SubFactory(
        TileFactory, game=factory.SelfAttribute("..source.game"),
    )
    amount = 1
