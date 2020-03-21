<template>
  <div>
    <navbar />
    <router-view></router-view>
  </div>
</template>

<script>
import Navbar from "./components/navbar.vue";

export default {
  components: {
    Navbar,
  },
};
</script>

<style scoped lang="scss">
.navbar {
  background-color: white;
}
.navbar-item {
  display: inline-block;
  margin: 0 4rem;
}
</style>
