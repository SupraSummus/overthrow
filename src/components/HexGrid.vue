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

export default {
  components: {
    HexTile,
  },
  data: () => {
    return {
      tiles: [
        { id: 0, x: 0, y: 0, z: 0 },
        { id: 1, x: 1, y: 1, z: -2 },
        { id: 2, x: 1, y: 0, z: -1 },
        { id: 4, x: 0, y: 1, z: -1 },
      ],
    };
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
