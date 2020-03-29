from hypothesis import assume, strategies

from overthrow.games.factories import PlayerFactory
from overthrow.games.models import Game, Tile, Movement


@strategies.composite
def boards(draw, max_radius=None):
    """ A game without any armies and players """
    radius = draw(strategies.integers(min_value=0, max_value=max_radius))
    return Game.generate_hexagonal(radius)


@strategies.composite
def games(
    draw,
    max_radius=2,
    min_player_count=0,
    max_player_count=5,
    unowned_tiles=True,
    unowned_armies=True,
    min_army_count=0,
    max_army_count=100,
    max_movement_count=None,
    max_movement_amount=10000,
):
    if min_player_count == 0 and not unowned_tiles:
        raise ValueError("Zero posible owners")

    game = draw(boards(max_radius=max_radius))

    # create players
    players = draw(strategies.lists(
        strategies.builds(
            PlayerFactory,
            game=strategies.just(game),
        ),
        min_size=min_player_count,
        max_size=max_player_count,
    ))

    # assign tiles and create armies
    owner_strategy = strategies.sampled_from([None] + players if unowned_tiles else players)
    army_count_strategy = strategies.integers(min_value=min_army_count, max_value=max_army_count)
    tiles = list(game.tiles.all())
    for tile in tiles:
        tile.owner = draw(owner_strategy)
        if tile.owner is not None or unowned_armies:
            tile.army = draw(army_count_strategy)
    Tile.objects.bulk_update(tiles, ['owner_id', 'army'])

    draw(movement_sets(
        source_tiles=tiles,
        target_tiles=tiles,
        max_movement_amount=max_movement_amount,
        max_movement_count=max_movement_count,
    ))

    return game


@strategies.composite
def movement_sets(
    draw,
    source_tiles, target_tiles,
    max_movement_amount=10000, max_movement_count=None,
):
    amount_strategy = strategies.integers(min_value=1, max_value=max_movement_amount)
    movements = draw(strategies.lists(
        strategies.builds(
            Movement,
            source=strategies.sampled_from(source_tiles),
            target=strategies.sampled_from(target_tiles),
            amount=amount_strategy,
        ),
        unique_by=lambda m: (m.source_id, m.target.id),
        max_size=max_movement_count,
    ))
    movements = [
        m
        for m in movements
        if m.source_id != m.target_id
    ]
    Movement.objects.bulk_create(movements)
    return movements


@strategies.composite
def coords(draw, max_radius=None):
    x = draw(strategies.integers(min_value=-max_radius, max_value=max_radius))
    y = draw(strategies.integers(min_value=-max_radius, max_value=max_radius))
    z = -x - y
    assume(z >= -max_radius and z <= max_radius)
    return (x, y, z)
