from django.urls import path
import rest_framework.authtoken.views

from . import views


urlpatterns = [
    path('api/token-auth/', rest_framework.authtoken.views.obtain_auth_token),
    path('api/user/', views.UserDetailAPIView.as_view()),
]
