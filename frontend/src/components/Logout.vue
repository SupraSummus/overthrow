<template>
  <div>
    <form @submit.prevent="logout">
      <p>Logged in as {{ $store.state.auth.username }}</p>
      <button type="submit">Logout</button>
    </form>
  </div>
</template>

<script>
export default {
  methods: {
    logout: function() {
      this.$store.commit("logged_out");
    },
  },
};
</script>
