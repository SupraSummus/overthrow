<template>
  <div>
    <navbar />
    <login v-if="!$store.getters.is_logged_in" />

    <router-view></router-view>
  </div>
</template>

<script>
import Navbar from "./components/navbar.vue";
import Login from "./components/Login.vue";

export default {
  components: {
    Navbar,
    Login,
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
