from rest_framework import generics
from django.shortcuts import get_object_or_404
from django.utils.functional import cached_property

from .models import Game
from . import serializers


class GameViewMixin:
    @cached_property
    def game(self):
        if 'id' in self.kwargs:
            return get_object_or_404(Game, id=self.kwargs['id'])
        else:
            return Game.objects.first()  # stub


class PlayerListView(GameViewMixin, generics.ListAPIView):
    serializer_class = serializers.PlayerSerializer

    def get_queryset(self):
        return self.game.players.all()


class TileListView(GameViewMixin, generics.ListAPIView):
    serializer_class = serializers.TileSerializer

    def get_queryset(self):
        tiles = self.game.tiles.all()
        return tiles
