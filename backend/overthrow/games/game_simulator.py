from collections import defaultdict, OrderedDict
import math

from .models import Movement
from . import coords


ATTACK_TO_DEFENSE_EFFICIENCY = 0.2
ATTACK_TO_ATTACK_EFFICIENCY = 0.2
DEFENSE_TO_ATTACK_EFFICIENCY = 0.25


def _add_to_dict_entry(d, path, v):
    """ d[path[0]][path[1]]..[path[N]] += v """
    for path_element in path[:-1]:
        d = d.setdefault(path_element, {})
    if path[-1] not in d:
        d[path[-1]] = 0
    d[path[-1]] += v


def zip_dicts(*dcts, default=None):
    for key in set(dcts[0]).union(*dcts[1:]):
        yield (key, tuple(d.get(key, default) for d in dcts))


class GameSimulator:
    def __init__(
        self,
        tiles,
        movements,
    ):
        self.tiles_by_id = {tile.id: tile for tile in tiles}
        self.tiles_by_coords = {tile.coords: tile for tile in self.tiles_by_id.values()}

        self.movements_by_id = {}
        self.movements_by_path = {}  # (source id, target id) -> movement
        self.movements_by_source = defaultdict(set)
        self.movements_by_next = defaultdict(set)
        self._total_outgoing_by_tile = {}  # tile id -> outgoing sum

        for movement in movements:
            self.add_movement(movement)

        self._movements_to_be_updated = OrderedDict()  # movement id -> True
        self._movements_to_be_deleted = {}  # path -> movement
        self.movements_to_be_created = []
        self.tiles_to_be_updated = set()  # ids of modified tiles

    def get_tile_total_outgoing(self, tile_id):
        if self._total_outgoing_by_tile.get(tile_id) is None:
            self._total_outgoing_by_tile[tile_id] = sum(
                movement.amount
                for movement in self.movements_by_source[tile_id]
            )
        return self._total_outgoing_by_tile[tile_id]

    def update_tile_owner(self, tile, owner_id):
        if tile.owner_id == owner_id:
            return
        for movement in list(self.movements_by_source[tile.id]):
            self.delete_movement(movement)
        tile.owner_id = owner_id
        self.tiles_to_be_updated.add(tile.id)

    def get_movement_path(self, movement):
        return (
            movement.source_id,
            movement.target_id,
        )

    def get_movement_effective_amount(self, movement):
        source_tile = self.tiles_by_id[movement.source_id]
        return math.floor(
            source_tile.army *
            movement.amount / self.get_tile_total_outgoing(movement.source_id)
        )

    def get_movement_next_tile(self, movement):
        source_tile = self.tiles_by_id[movement.source_id]
        target_tile = self.tiles_by_id[movement.target_id]
        next_tile = self.tiles_by_coords[coords.next_on_path(
            source_tile.coords,
            target_tile.coords,
        )]
        return next_tile

    def get_movement_owner(self, movement):
        return self.tiles_by_id[movement.source_id].owner_id

    def create_movement(self, path, amount):
        if path in self._movements_to_be_deleted:
            movement = self._movements_to_be_deleted[path]
            del self._movements_to_be_deleted[path]
            movement.amount = amount
            self._movements_to_be_updated[movement.id] = True
        else:
            movement = Movement(
                source_id=path[0],
                target_id=path[1],
                amount=amount,
            )
            self.movements_to_be_created.append(movement)
        self.add_movement(movement)

    def add_movement(self, movement):
        next_tile = self.get_movement_next_tile(movement)
        path = self.get_movement_path(movement)

        self._total_outgoing_by_tile[movement.source_id] = None
        assert movement.id not in self.movements_by_id
        self.movements_by_id[movement.id] = movement
        assert path not in self.movements_by_path
        self.movements_by_path[path] = movement
        self.movements_by_source[movement.source_id].add(movement)
        self.movements_by_next[next_tile.id].add(movement)

    def delete_movement(self, movement):
        next_tile = self.get_movement_next_tile(movement)
        path = self.get_movement_path(movement)
        assert path not in self._movements_to_be_deleted
        self._movements_to_be_deleted[path] = movement

        self._total_outgoing_by_tile[movement.source_id] = None
        del self.movements_by_path[path]
        del self.movements_by_id[movement.id]
        self.movements_by_source[movement.source_id].remove(movement)
        self.movements_by_next[next_tile.id].remove(movement)

        if movement.id in self._movements_to_be_updated:
            del self._movements_to_be_updated[movement.id]

    def update_movement_amount(self, movement, amount_delta):
        self._total_outgoing_by_tile[movement.source_id] = None
        movement.amount += amount_delta
        if movement.amount > 0:
            self._movements_to_be_updated[movement.id] = True
            return movement
        else:
            self.delete_movement(movement)
            return None

    @property
    def movements_to_be_updated(self):
        return [
            self.movements_by_id[movement_id]
            for movement_id in self._movements_to_be_updated.keys()
        ]

    @property
    def movements_to_be_deleted(self):
        return [
            m.id
            for m in self._movements_to_be_deleted.values()
        ]

    def simulate(self):
        self.simulate_battles()
        self.simulate_owner_changes()
        self.simulate_movements()

    def simulate_battles(self):
        tile_defending_armies = {}  # tile id -> amount
        tile_attacking_armies = {}  # tile id -> source tile id -> amount

        # intially all armies defend
        for tile in self.tiles_by_id.values():
            tile_defending_armies[tile.id] = tile.army

        # dispatch armies, gather incoming movements
        for movement in self.movements_by_id.values():
            source_tile = self.tiles_by_id[movement.source_id]
            target_tile = self.tiles_by_id[movement.target_id]
            movement_amount = min(movement.amount, tile_defending_armies[movement.source_id])
            tile_defending_armies[movement.source_id] -= movement_amount
            next_tile_coords = coords.next_on_path(source_tile.coords, target_tile.coords)
            next_tile = self.tiles_by_coords[next_tile_coords]
            _add_to_dict_entry(
                tile_attacking_armies,
                (next_tile.id, movement.source_id),
                movement_amount,
            )

        # calculate defender losses, decrease attacking armies accordingly
        for tile_id, attacking_armies in tile_attacking_armies.items():
            tile = self.tiles_by_id[tile_id]
            force = sum(
                amount
                for source_tile_id, amount in attacking_armies.items()
                if self.tiles_by_id[source_tile_id].owner_id != tile.owner_id
            ) * ATTACK_TO_DEFENSE_EFFICIENCY
            losses = min(force, tile.army)
            if losses > 0:
                tile.army -= losses
                self.tiles_to_be_updated.add(tile_id)
                for source_tile_id, amount in attacking_armies.items():
                    source_tile = self.tiles_by_id[source_tile_id]
                    if source_tile.owner_id != tile.owner_id:
                        attacking_armies[source_tile_id] -= force / losses * amount

        # calculate attacker losses
        for target_tile_id, attacking_armies in tile_attacking_armies.items():
            target_tile = self.tiles_by_id[target_tile_id]
            attacking_armies_sum = sum(attacking_armies.values())

            # index by player
            attacking_armies_by_player = defaultdict(int)
            for source_tile_id, amount in attacking_armies.items():
                source_tile = self.tiles_by_id[source_tile_id]
                attacking_armies_by_player[source_tile.owner_id] += amount

            for source_tile_id, receiving_army in attacking_armies.items():
                source_tile = self.tiles_by_id[source_tile_id]
                deaths = sum(
                    (
                        (dealing_army * ATTACK_TO_ATTACK_EFFICIENCY)  # dealing army force
                        * receiving_army / (attacking_armies_sum - dealing_army)  # receiving army share
                    )
                    for player_id, dealing_army in attacking_armies_by_player.items()
                    if (
                        player_id != source_tile.owner_id and
                        attacking_armies_sum - dealing_army > 0
                    )
                )

                # received from defenders
                defenders_defending_against = attacking_armies_sum - attacking_armies_by_player[target_tile.owner_id]
                if source_tile.owner_id != target_tile.owner_id and defenders_defending_against > 0:
                    deaths += (
                        (tile_defending_armies[target_tile_id] * DEFENSE_TO_ATTACK_EFFICIENCY) *
                        receiving_army / defenders_defending_against
                    )

                deaths = min(deaths, source_tile.army)
                if deaths > 0:
                    source_tile.army -= deaths
                    self.tiles_to_be_updated.add(source_tile_id)

        for tile_id in self.tiles_to_be_updated:
            tile = self.tiles_by_id[tile_id]
            tile.army = int(math.floor(tile.army))

    def simulate_owner_changes(self):
        for next_tile_id, movement_ids in self.movements_by_next.items():
            parties = set()
            tile = self.tiles_by_id[next_tile_id]
            if tile.army > 0:
                parties.add(tile.owner_id)
            for movement_id in movement_ids:
                amount = self.get_movement_effective_amount(movement_id)
                if amount > 0:
                    parties.add(self.get_movement_owner(movement_id))
            if len(parties) == 1:
                new_owner_id = list(parties)[0]
                self.update_tile_owner(tile, new_owner_id)

    def simulate_movements(self):
        armies_in = defaultdict(int)
        armies_out = defaultdict(int)
        movement_delta_by_path = defaultdict(int)

        for movement in list(self.movements_by_id.values()):
            source_tile = self.tiles_by_id[movement.source_id]
            target_tile = self.tiles_by_id[movement.target_id]
            next_tile = self.tiles_by_coords[coords.next_on_path(
                source_tile.coords,
                target_tile.coords,
            )]
            amount = min(source_tile.army - armies_out[movement.source_id], movement.amount)

            if (
                amount == 0 or
                source_tile.owner_id != next_tile.owner_id
            ):
                # couldnt execute the movement
                continue

            # execute
            armies_out[movement.source_id] += amount
            armies_in[next_tile.id] += amount
            movement_delta_by_path[self.get_movement_path(movement)] -= amount

            # there are more tiles to go - create/update movements for them
            if next_tile.id != target_tile.id:
                next_path = (next_tile.id, movement.target_id)
                movement_delta_by_path[next_path] += amount

        # update movements
        for path, delta in movement_delta_by_path.items():
            if delta == 0:
                continue
            movement = self.movements_by_path.get(path)
            if movement is None:
                self.create_movement(path, delta)
            else:
                self.update_movement_amount(movement, delta)

        # update tiles
        for tile_id, (incoming, outgoing) in zip_dicts(armies_in, armies_out, default=0):
            delta = incoming - outgoing
            if delta == 0:
                continue
            tile = self.tiles_by_id[tile_id]
            tile.army += delta
            self.tiles_to_be_updated.add(tile_id)
