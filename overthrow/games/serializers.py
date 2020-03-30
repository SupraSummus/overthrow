from rest_framework import serializers

from .models import Tile, Player, Movement


class PlayerSerializer(serializers.ModelSerializer):
    class Meta:
        model = Player
        fields = ['id', 'user']


class TileSerializer(serializers.ModelSerializer):
    class Meta:
        model = Tile
        fields = ['id', 'x', 'y', 'z', 'owner', 'army']


class MovementSerializer(serializers.ModelSerializer):
    class Meta:
        model = Movement
        fields = ['id', 'source', 'target', 'amount']
        read_only_fields = ['source']
