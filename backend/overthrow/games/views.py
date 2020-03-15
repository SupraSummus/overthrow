from rest_framework import generics, permissions
from django.shortcuts import get_object_or_404
from django.utils.functional import cached_property

from .models import Game, Tile, Movement
from . import serializers


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


class TileCommandPermission(permissions.BasePermission):
    def has_object_permission(self, request, view, obj):
        if not obj.owner:
            return False
        return request.user == obj.owner.user


class MoveAPIView(TileViewMixin, generics.CreateAPIView):
    permission_classes = [TileCommandPermission]
    serializer_class = serializers.MovementSerializer

    def post(self, request, *args, **kwargs):
        self.check_object_permissions(request, self.tile)
        return super().post(request, *args, **kwargs)

    def perform_create(self, serializer):
        serializer.save(source=self.tile)


class MoveListAPIView(GameViewMixin, generics.ListAPIView):
    serializer_class = serializers.MovementSerializer

    def get_queryset(self):
        return Movement.objects.filter(
            source__game=self.game,
            source__owner__user=self.request.user,
        )
