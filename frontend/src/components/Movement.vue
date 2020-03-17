<template>
  <div
    v-if="show"
    class="movement"
    v-bind:style="{
      transform: this.position_transform,
    }"
    v-on:click="$emit('delete')"
  >
    <div class="movement-content">
      {{ amount }}
    </div>
    <div
      class="movement-arrow"
      v-bind:style="{
        transform: `translate(-50%, -50%) rotate(${this.rotation + 90}deg)`,
      }"
    >
      &lt;
    </div>
  </div>
</template>

<script>
import { coord_delta_one, coord_string } from "@/coord";
import { tile_height, tile_width, delta_rotations } from "@/constants";

export default {
  props: ["source", "target", "amount", "show"],
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
    delta: function() {
      return coord_delta_one(this.source, this.target);
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
.movement-arrow {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  cursor: pointer;
}
.movement-arrow {
  font-weight: bold;
  color: lightgray;
  z-index: -1;
  font-size: 50px;
}
</style>
