from hypothesis import assume, strategies

from overthrow.games.models import Game


max_radius = 3


@strategies.composite
def board(draw):
    """ A game without any armies and players """
    radius = draw(strategies.integers(min_value=0, max_value=max_radius))
    return Game.generate_hexagonal(radius)


@strategies.composite
def coords(draw):
    x = draw(strategies.integers(min_value=-max_radius, max_value=max_radius))
    y = draw(strategies.integers(min_value=-max_radius, max_value=max_radius))
    z = -x - y
    assume(z >= -max_radius and z <= max_radius)
    return (x, y, z)
