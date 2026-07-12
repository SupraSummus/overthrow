from rest_framework import serializers
from rest_framework_recaptcha.fields import ReCaptchaField

from .models import User


class UserSerializer(serializers.ModelSerializer):
    class Meta:
        model = User
        fields = ["id", "username", "email", "password", "recaptcha"]
        extra_kwargs = {"password": {"write_only": True}}

    recaptcha = ReCaptchaField()

    def save(self, **kwargs):
        del self.validated_data["recaptcha"]
        User.objects.create_user(
            **self.validated_data, **kwargs,
        )
