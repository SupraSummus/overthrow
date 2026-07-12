from django.contrib.auth.models import AbstractUser

from overthrow.utils import UUIDModel


class User(AbstractUser, UUIDModel):
    pass
