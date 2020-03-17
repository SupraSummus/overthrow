<template>
  <div class="on-map" :style="{ transform: `translate(${left}px, ${top}px)` }">
    <slot />
  </div>
</template>

<script>
import { tile_height, tile_width } from "@/constants";
export default {
  props: ["x", "y", "z"],
  computed: {
    top: function() {
      return this.y * tile_height * 0.75;
    },
    left: function() {
      return (this.x + this.y / 2) * tile_width;
    },
  },
};
</script>

<style scoped>
.on-map {
  top: 0;
  left: 0;
  position: absolute;
}
</style>
