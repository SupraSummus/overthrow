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
        @zoom="
          console.log('zoom');
          $nextTick(() => {
            if ($refs.move_menu_slider) $refs.move_menu_slider.refresh();
          });
        "
      >
        <div class="grid">
          <map-tile
            v-for="(tile, tile_id) in tiles"
            v-bind:key="tile_id"
            v-bind="tile"
            v-on:select="on_select_tile(tile)"
            v-on:hover="on_hover_tile(tile)"
            v-on:contextmenu.native.prevent="on_contextmenu_tile(tile)"
          />
          <movement-chain
            v-for="(movement, movement_id) in movements"
            v-bind:key="movement_id"
            v-bind="movement"
            v-on:delete="delete_movement(movement_id)"
          />

          <!-- hover path -->
          <movement-chain
            v-if="selected_tile && selected_tile.owned && hovered_tile"
            v-bind:source="selected_tile"
            v-bind:target="hovered_tile"
            v-bind:amount="null"
          />

          <!-- movement command context menu -->
          <context-dialog v-if="move_menu_tile" v-bind="move_menu_tile">
            <vue-slider
              ref="move_menu_slider"
              :value="0"
              :adsorb="true"
              :interval="1"
              :min="0"
              :max="selected_tile.army"
              :lazy="true"
              :marks="true"
              @change="move"
              @mousedown.native.stop
            />
          </context-dialog>
        </div>
      </panZoom>
    </div>
  </div>
</template>

<script>
import Vue from "vue";

import MapTile from "./MapTile.vue";
import MovementChain from "./MovementChain.vue";
import ContextDialog from "./context_dialog.vue";
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
    MovementChain,
    ContextDialog,
  },
  props: ["id"],
  data: () => {
    return {
      players: {}, // player id -> player
      tiles: {}, // tile id -> tile
      movements: {}, // movement id > movement

      // indexes
      tile_ids_by_coord: {}, // tile coord string -> tile id

      // UI state
      selected_tile_id: null,
      hovered_tile: null,
      move_menu_tile: null,
      move_amount: 0,

      //debug
      console: window.console,
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
    selected_tile: function() {
      if (this.selected_tile_id) {
        return this.tiles[this.selected_tile_id];
      } else {
        return null;
      }
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
          army: null,
          owner: null, // owner id

          // relations
          outgoing_movements: {}, // movement id -> movement object

          // computed
          selected: false,
          owned: false,
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
          processing: false,
        });
      }
      const movement = this.movements[id];

      if (movement.source == null || movement.source.id != source) {
        if (movement.source != null) {
          Vue.delete(movement.source.outgoing_movements, movement.id);
        }
        movement.source = this.get_or_create_tile(source);
        Vue.set(movement.source.outgoing_movements, movement.id, movement);
      }

      movement.target = this.get_or_create_tile(target);
      movement.amount = amount;
    },

    delete_movement: function(id) {
      const movement = this.movements[id];
      movement.processing = true;
      call_api({
        method: "DELETE",
        path: `movement/${id}/`,
      }).then(() => {
        Vue.delete(movement.source.outgoing_movements, id);
        Vue.delete(this.movements, id);
      });
    },

    move: function(amount) {
      console.log("move", amount);
      if (amount != 0) {
        call_api({
          method: "POST",
          path: `tile/${this.selected_tile.id}/move/`,
          payload: {
            target: this.hovered_tile.id,
            amount: amount,
          },
        }).then(movement => this.load_movement(movement));
      }

      this.selected_tile_id = null;
      this.hovered_tile = null;
      this.move_menu_tile = null;
    },

    on_select_tile: function(tile) {
      const tile_id = tile.id;

      if (this.move_menu_tile) {
        // clickaway from contextment
        this.move_menu_tile = null;
        this.selected_tile_id = tile_id;
        this.hovered_tile = tile;
        return;
      }

      if (!this.selected_tile_id) {
        // nothing was selected -> select this
        this.selected_tile_id = tile_id;
        return;
      }

      if (this.selected_tile_id == tile_id) {
        // unselect selected
        this.selected_tile_id = null;
      } else if (!this.tiles[this.selected_tile_id].owned) {
        // selected foreign tile -> change selection to this
        this.selected_tile_id = tile_id;
      } else {
        // selected own tile -> command movement to this tile
        this.move(this.selected_tile.army);
      }
    },

    on_hover_tile: function(tile) {
      if (!this.move_menu_tile) {
        // no hover when contextmenu is enabled
        this.hovered_tile = tile;
      }
    },

    on_contextmenu_tile: function(tile) {
      // open move contextmenu
      if (
        this.selected_tile &&
        this.selected_tile.id != tile.id &&
        this.selected_tile.owned
      ) {
        this.move_menu_tile = tile;
        this.hovered_tile = tile;
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
