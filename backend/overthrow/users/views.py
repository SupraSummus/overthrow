from rest_framework import generics

from . import serializers


class UserDetailAPIView(generics.RetrieveAPIView):
    serializer_class = serializers.UserSerializer

    def get_object(self):
        return self.request.user
