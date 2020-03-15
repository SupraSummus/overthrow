<template>
  <div>
    <div class="grid-container">
      <panZoom selector=".grid" :options="{ bounds: false }">
        <div class="grid">
          <hex-tile
            v-for="(tile, tile_coord_id) in tiles"
            v-bind:key="tile_coord_id"
            v-bind:tile="tile"
            v-bind:players="players"
          >
          </hex-tile>
        </div>
      </panZoom>
    </div>
  </div>
</template>

<script>
import Vue from "vue";

import HexTile from "./HexTile.vue";
import call_api from "../api";

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
    HexTile,
  },
  props: ["id"],
  data: () => {
    return {
      players: {},
      tiles: {},
    };
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
    set_tile: function(tile) {
      tile.borders = {};
      Vue.set(this.tiles, get_tile_coord_id(tile), tile);
      coord_neighbour_deltas.forEach(delta => {
        this.update_border(tile, delta);
        this.update_border(coord_sum(tile, delta), coord_negative(delta));
      });
    },

    update_border: function(tile, delta) {
      const tile_id = get_tile_coord_id(tile);
      const neighbour_id = get_tile_coord_id(coord_sum(tile, delta));

      // nonexistent tile
      if (!(tile_id in this.tiles)) return;

      let show_border = false;
      if (neighbour_id in this.tiles) {
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
  },
};
</script>

<style scoped>
.grid-container {
  position: fixed;
  z-index: -1;
  border: 0;
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
