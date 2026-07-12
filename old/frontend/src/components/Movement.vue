<template>
  <div
    class="movement"
    v-bind:style="{
      transform: this.position_transform,
    }"
  >
    <div class="movement-content">
      {{ amount }}
    </div>
    <spinner v-if="processing > 0" class="movement-spinner" />
    <div
      class="movement-arrow"
      v-bind:style="{
        transform: `translate(-50%, -50%) rotate(${this.rotation - 90}deg)`,
      }"
    >
      ⇨
    </div>
  </div>
</template>

<script>
import Spinner from "vue-simple-spinner";

import { coord_string } from "@/coord";
import { tile_height, tile_width, delta_rotations } from "@/constants";

export default {
  props: ["source", "delta", "amount", "processing"],
  components: { Spinner },
  computed: {
    top: function() {
      return (this.source.y + this.delta.y / 2) * tile_height * 0.75 + "px";
    },
    left: function() {
      return (
        (this.source.x +
          this.delta.x / 2 +
          this.source.y / 2 +
          this.delta.y / 4) *
          tile_width +
        "px"
      );
    },
    position_transform: function() {
      return `translate(${this.left}, ${this.top})`;
    },
    rotation: function() {
      return delta_rotations[coord_string(this.delta)];
    },
  },
};
</script>

<style>
.movement {
  top: 0;
  left: 0;
  position: absolute;
}
.movement-content,
.movement-arrow,
.movement-spinner {
  font-weight: bold;
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  cursor: pointer;
}
.movement-arrow {
  font-size: 60px;
  color: lightgray;
  z-index: -1;
}
.movement-spinner: {
  background-color: rgba(255, 255, 255, 0.5);
}
</style>
