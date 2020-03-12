<template>
  <div class="tile" :style="position_style">
    <div class="content">hex tile {{ tile.x }} {{ tile.y }} <slot></slot></div>
  </div>
</template>

<script>
const tile_size = 100; // px
const sqrt3 = Math.sqrt(3);

export default {
  props: ["tile"],
  computed: {
    position_style: function() {
      return {
        position: "absolute",
        top: ((this.tile.y - 1) * tile_size) / sqrt3 + "px",
        left: (this.tile.x + this.tile.y / 2 - 0.5) * tile_size + "px",
        width: tile_size + "px",
        height: (tile_size / sqrt3) * 2 + "px",
      };
    },
  },
};
</script>

<style scoped>
.content {
  margin: 0;
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
}
</style>
