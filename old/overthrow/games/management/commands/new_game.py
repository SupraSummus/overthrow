from django.core.management.base import BaseCommand
from overthrow.games.models import Game


class Command(BaseCommand):
    help = "Create a new game"

    def add_arguments(self, parser):
        parser.add_argument("radius", type=int)

    def handle(self, radius, **kwargs):
        game = Game.generate_hexagonal(radius)
        print(game.id)
