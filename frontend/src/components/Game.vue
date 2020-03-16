<template>
  <div>
    <div>player_id: {{ player_id }}</div>
    <div class="grid-container">
      <panZoom
        selector=".grid"
        :options="{
          bounds: false,
          onTouch: function() {
            // returning false causes panzoom to propagate events down
            return false;
          },
        }"
      >
        <div class="grid">
          <map-tile
            v-for="(tile, tile_id) in tiles"
            v-bind:key="tile_id"
            v-bind="tile"
            v-on:select="select_tile(tile.id)"
          />
          <movement
            v-for="(movement, movement_id) in movements"
            v-bind:key="movement_id"
            v-bind="movement"
          />
        </div>
      </panZoom>
    </div>
  </div>
</template>

<script>
import Vue from "vue";

import MapTile from "./MapTile.vue";
import Movement from "./Movement.vue";
import call_api from "@/api";
import {
  coord_string as get_tile_coord_id,
  coord_sum,
  coord_neighbour_deltas,
  coord_negative,
} from "@/coord";

export default {
  components: {
    MapTile,
    Movement,
  },
  props: ["id"],
  data: () => {
    return {
      players: {}, // player id -> player
      tiles: {}, // tile id -> tile
      tile_ids_by_coord: {}, // tile coord string -> tile id
      selected_tile_id: null,
      movements: {}, // movement id > movement
    };
  },
  computed: {
    player_id: function() {
      const user_id = this.$store.state.user.id;
      for (let player_id in this.players) {
        if (this.players[player_id].user == user_id) return player_id;
      }
      return null;
    },
  },
  created: function() {
    call_api({
      path: `game/${this.id}/tiles/`,
      method: "GET",
    }).then(resp => {
      resp.forEach(tile => {
        this.set_tile(tile);
      });
    });
    call_api({
      path: `game/${this.id}/players/`,
      method: "GET",
    }).then(resp => {
      resp.forEach(player => {
        Vue.set(this.players, player.id, player);
      });
    });
  },
  methods: {
    get_or_create_tile: function(id) {
      if (!(id in this.tiles)) {
        Vue.set(this.tiles, id, {
          id,
          x: null,
          y: null,
          z: null,
          amount: null,
          owner: null, // owner id
          selected: false,
          owned: false,
          outgoing_movements: {}, // movement id -> movement object
          borders: [], // set of delta coord strings to show border on
        });
      }
      return this.tiles[id];
    },

    set_tile: function(tile_data) {
      const tile = this.get_or_create_tile(tile_data.id);

      const coord_string = get_tile_coord_id(tile_data);
      this.tile_ids_by_coord[coord_string] = tile.id;

      // copy tile data into tile
      Object.assign(tile, tile_data);

      // computed properties
      tile.selected = tile.id == this.selected_tile_id;
      tile.owned = tile.owner == this.player_id;

      // compute borders
      coord_neighbour_deltas.forEach(delta => {
        this.update_border(tile, delta);
        const neighbour_coord_string = get_tile_coord_id(
          coord_sum(tile, delta),
        );
        if (neighbour_coord_string in this.tile_ids_by_coord) {
          this.update_border(
            this.tiles[this.tile_ids_by_coord[neighbour_coord_string]],
            coord_negative(delta),
          );
        }
      });
    },

    update_border: function(tile, delta) {
      const delta_string = get_tile_coord_id(delta);

      // remove existing border
      const i = tile.borders.indexOf(delta_string);
      if (i > -1) tile.borders.splice(i, 1);

      // add new border if needed
      const neighbour_coord_id = get_tile_coord_id(coord_sum(tile, delta));
      if (neighbour_coord_id in this.tile_ids_by_coord) {
        const neighbour_id = this.tile_ids_by_coord[neighbour_coord_id];
        if (
          tile.owner !== null && // owned by someone
          tile.owner !== this.tiles[neighbour_id].owner // owned by different player than neighbour
        ) {
          tile.borders.push(delta_string);
        }
      }
    },

    load_movement: function({ id, source, target, amount }) {
      if (!(id in this.movements)) {
        Vue.set(this.movements, id, {
          id: id,
          show: true,
          source: null,
          target: null,
          amount: null,
        });
      }
      const movement = this.movements[id];

      if (movement.source == null || movement.source.id != source) {
        if (movement.source != null) {
          delete movement.source.outgoing_movements[movement.id];
        }
        movement.source = this.get_or_create_tile(source);
        movement.source.outgoing_movements[movement.id] = movement;
      }

      movement.target = this.get_or_create_tile(target);
      movement.amount = amount;
    },

    delete_movement: function(id) {
      const movement = this.movements[id];
      if (movement.source) {
        delete this.tiles[movement.source].outgoing_movements[id];
      }
      delete this.movements[id];
    },

    select_tile: function(tile_id) {
      if (!this.selected_tile_id) {
        // nothing was selected -> select this
        this.selected_tile_id = tile_id;
      } else if (this.selected_tile_id == tile_id) {
        // unselect selected
        this.selected_tile_id = null;
      } else if (!this.tiles[this.selected_tile_id].owned) {
        // selected foreign tile -> change selection to this
        this.selected_tile_id = tile_id;
      } else {
        // selected own tile -> command movement to this tile
        call_api({
          method: "POST",
          path: `tile/${this.selected_tile_id}/move/`,
          payload: {
            target: tile_id,
            amount: this.tiles[this.selected_tile_id].army,
          },
        }).then(movement => this.load_movement(movement));
        this.selected_tile_id = null;
      }
    },
  },
  watch: {
    selected_tile_id: function(after_id, before_id) {
      if (before_id in this.tiles) this.tiles[before_id].selected = false;
      if (after_id in this.tiles) this.tiles[after_id].selected = true;
    },
    player_id: function(player_id) {
      for (let tile_id in this.tiles) {
        this.tiles[tile_id].owned = this.tiles[tile_id].owner == player_id;
      }
    },
    id: {
      immediate: true,
      handler(game_id) {
        call_api({
          method: "GET",
          path: `game/${game_id}/movements/`,
        }).then(movements => {
          for (let movement_id in this.movements)
            this.delete_movement[movement_id];
          movements.forEach(movement => this.load_movement(movement));
        });
      },
    },
  },
};
</script>

<style scoped lang="scss">
.grid-container {
  position: fixed;
  z-index: -1;
  overflow: hidden;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
}
.grid {
  position: relative;
}
</style>
