<template>
  <on-map v-bind="$props">
    <div class="dialog">
      <slot />
    </div>
  </on-map>
</template>

<script>
import OnMap from "./on_map.vue";

export default {
  props: ["x", "y", "z"],
  components: { OnMap },
};
</script>

<style scoped>
.dialog {
  position: absolute;
  transform: translate(-50%, -50%);

  background-color: white;
  padding: 1rem;
  box-shadow: 0 0 2px;
  width: 200px;
}
</style>
