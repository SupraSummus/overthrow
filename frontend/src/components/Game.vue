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
          >
          </map-tile>
        </div>
      </panZoom>
    </div>
  </div>
</template>

<script>
import Vue from "vue";

import MapTile from "./MapTile.vue";
import call_api from "@/api";

const coord_neighbour_deltas = [
  { x: -1, y: 0, z: 1 },
  { x: -1, y: 1, z: 0 },
  { x: 0, y: -1, z: 1 },
  { x: 0, y: 1, z: -1 },
  { x: 1, y: -1, z: 0 },
  { x: 1, y: 0, z: -1 },
];

function coord_sum(a, b) {
  return { x: a.x + b.x, y: a.y + b.y, z: a.z + b.z };
}

function coord_negative(a) {
  return { x: -a.x, y: -a.y, z: -a.z };
}

function get_tile_coord_id(coords) {
  return `${coords.x}_${coords.y}_${coords.z}`;
}

export default {
  components: {
    MapTile,
  },
  props: ["id"],
  data: () => {
    return {
      players: {}, // player id -> player
      tiles: {}, // tile id -> tile
      tile_ids_by_coord: {}, // tile coord string -> tile id
      selected_tile_id: "",
      movements: {}, // movement id > movement
    };
  },
  computed: {
    player_id: function() {
      const user_id = this.$store.state.user.id;
      for (let player_id in this.players) {
        if (this.players[player_id].user == user_id) return player_id;
      }
      return "";
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
    get_or_create_tile: function (id) {
      if (!(id in this.tiles)) {
        this.tiles[id] = {id: id};
      }
      return this.tiles[id];
    }
    set_tile: function(tile_data) {
      const tile = this.get_or_create_tile(tile_data.id);

      const coord_string = get_tile_coord_id(tile_data);
      this.tile_ids_by_coord[coord_stirng] = id;

      // copy tile data into tile
      Object.assign(tile, tile_data);

      // computed properties
      tile.selected = tile.id == this.selected_tile_id;
      tile.owned = tile.owner == this.player_id;

      // compute borders
      tile.borders = {};
      coord_neighbour_deltas.forEach(delta => {
        this.update_border(tile, delta);
        this.update_border(coord_sum(tile, delta), coord_negative(delta));
      });
    },

    update_border: function(tile, delta) {
      const tile_coord_id = get_tile_coord_id(tile);
      if (!(tile_coord_id in this.tile_ids_by_coord)) {
        // nonexistent tile
        return;
      }
      const tile_id = this.tile_ids_by_coord[tile_coord_id];

      const neighbour_coord_id = get_tile_coord_id(coord_sum(tile, delta));

      let show_border = false;
      if (neighbour_coord_id in this.tile_ids_by_coord) {
        const neighbour_id = this.tile_ids_by_coord[neighbour_coord_id];
        show_border =
          this.tiles[tile_id].owner !== this.tiles[neighbour_id].owner &&
          this.tiles[tile_id].owner !== null;
      }

      Vue.set(
        this.tiles[tile_id].borders,
        get_tile_coord_id(delta),
        show_border,
      );
    },

    set_movement: function({ id, source, target, amount }) {
      if (!(id in this.movements)) this.movements[id] = {id: id};
      const movement= this.movements[id];

      if (movement.source != source) {
        if (movement.source){
          delete this.tiles[movement.source].outgoing_movements[movement.id];
        }
        movement.source = source;
        if (movement.source) {
          this.get_or_create_tile(movement.source).outgoing_movements[movement.id] = movement;
        }
      }

      movement.target = target;
      movement.amount = amount;
    },

    select_tile: function(tile_id) {
      if (!this.selected_tile_id) {
        // nothing was selected -> select this
        this.selected_tile_id = tile_id;
      } else if (this.selected_tile_id == tile_id) {
        // unselect selected
        this.selected_tile_id = "";
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
        }).then(movement => console.log(movement));
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
            delete this.movements[movement_id];
          movements.forEach(movement => this.add_movement(movement));
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
