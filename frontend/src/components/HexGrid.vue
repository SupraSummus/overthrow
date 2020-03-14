<template>
  <div>
    hex grid
    <div class="grid-container">
      <panZoom selector=".grid" :options="{ bounds: false }">
        <div class="grid">
          <hex-tile
            v-for="tile in tiles"
            v-bind:key="tile.id"
            v-bind:tile="tile"
            >asdf</hex-tile
          >
        </div>
      </panZoom>
    </div>
  </div>
</template>

<script>
import HexTile from "./HexTile.vue";
import call_api from "../api";

export default {
  components: {
    HexTile,
  },
  data: () => {
    return {
      tiles: [],
    };
  },
  created: function() {
    call_api({
      path: "tiles/",
      method: "GET",
    }).then(resp => {
      this.tiles = resp;
    });
  },
};
</script>

<style scoped>
.grid-container {
  border: 1px solid black;
  overflow: hidden;
  width: 90vw;
  height: 90vw;
}
.grid {
  position: relative;
}
</style>
