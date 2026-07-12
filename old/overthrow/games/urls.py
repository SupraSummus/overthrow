from django.urls import path

from . import views

urlpatterns = [
    # until we have multi-game aware frontend
    path("api/game/default/tiles/", views.TileListView.as_view()),
    path("api/game/default/players/", views.PlayerListView.as_view()),
    path("api/game/default/movements/", views.MoveListAPIView.as_view()),
    path("api/game/<uuid:id>/tiles/", views.TileListView.as_view()),
    path("api/game/<uuid:id>/players/", views.PlayerListView.as_view()),
    path("api/tile/<uuid:id>/move/", views.MoveAPIView.as_view()),
    path("api/game/<uuid:id>/movements/", views.MoveListAPIView.as_view()),
    path("api/movement/<uuid:pk>/", views.MovementAPIView.as_view()),
]
