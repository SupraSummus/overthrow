from django.urls import path
import rest_framework.authtoken.views


urlpatterns = [
    path('api/token-auth/', rest_framework.authtoken.views.obtain_auth_token),
]
