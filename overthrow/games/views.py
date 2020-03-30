from rest_framework import generics
from django.shortcuts import get_object_or_404
from django.utils.functional import cached_property

from .models import Game, Tile, Movement
from . import serializers, permissions


class GameViewMixin:
    @cached_property
    def game(self):
        if 'id' in self.kwargs:
            return get_object_or_404(Game, id=self.kwargs['id'])
        else:
            # stub. until we have multi-game aware frontend
            return Game.objects.first()


class PlayerListView(GameViewMixin, generics.ListAPIView):
    serializer_class = serializers.PlayerSerializer

    def get_queryset(self):
        return self.game.players.all()


class TileListView(GameViewMixin, generics.ListAPIView):
    serializer_class = serializers.TileSerializer

    def get_queryset(self):
        tiles = self.game.tiles.all()
        return tiles


class TileViewMixin:
    @cached_property
    def tile(self):
        return get_object_or_404(Tile, id=self.kwargs['id'])


class MoveAPIView(TileViewMixin, generics.CreateAPIView):
    permission_classes = [permissions.TileCommandPermission]
    serializer_class = serializers.MovementSerializer

    def post(self, request, *args, **kwargs):
        self.check_object_permissions(request, self.tile)
        return super().post(request, *args, **kwargs)

    def perform_create(self, serializer):
        serializer.save(source=self.tile)


class MovementAPIView(generics.RetrieveUpdateDestroyAPIView):
    permission_classes = [permissions.MovementCommandPermission]
    queryset = Movement.objects.all()
    serializer_class = serializers.MovementSerializer


class MoveListAPIView(GameViewMixin, generics.ListAPIView):
    serializer_class = serializers.MovementSerializer

    def get_queryset(self):
        return Movement.objects.filter(
            source__game=self.game,
            source__owner__user=self.request.user,
        )
