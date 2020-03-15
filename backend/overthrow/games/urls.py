from django.urls import path

from . import views

urlpatterns = [
    path('api/game/default/tiles/', views.TileListView.as_view()),
    path('api/game/<uuid:id>/tiles/', views.TileListView.as_view()),
    path('api/game/default/players/', views.PlayerListView.as_view()),
    path('api/game/<uuid:id>/players/', views.PlayerListView.as_view()),
]
