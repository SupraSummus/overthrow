from rest_framework import permissions


class TileCommandPermission(permissions.BasePermission):
    def has_object_permission(self, request, view, obj):
        if not obj.owner:
            return False
        return request.user == obj.owner.user


class MovementCommandPermission(permissions.BasePermission):
    def has_object_permission(self, request, view, obj):
        movement = obj
        tile = movement.source
        if not tile.owner:
            return False
        return request.user == tile.owner.user
