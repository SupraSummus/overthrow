from rest_framework import generics

from .models import Game
from . import serializers


class TileListView(generics.ListAPIView):
    serializer_class = serializers.TileSerializer

    def get_queryset(self):
        game = Game.objects.first()  # stub
        tiles = game.tiles.all()
        return tiles
