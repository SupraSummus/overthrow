from django.urls import path

from . import views

urlpatterns = [
    path('api/tiles/', views.TileListView.as_view()),
]
