from rest_framework import generics
from rest_framework.views import APIView
from rest_framework.response import Response
from django.conf import settings

from . import serializers


class UserDetailAPIView(generics.RetrieveAPIView):
    serializer_class = serializers.UserSerializer

    def get_object(self):
        return self.request.user


class RegisterAPIView(generics.CreateAPIView):
    serializer_class = serializers.UserSerializer


class RecaptchaAPIView(APIView):
    def get(self, request, format=None):
        return Response({
            'site_key': settings.RECAPTCHA_SITE_KEY,
        })
