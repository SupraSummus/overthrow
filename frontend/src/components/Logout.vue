<template>
  <div>
    <form @submit.prevent="logout">
      <p>
        Logged in as {{ $store.state.user.username }}
        <button type="submit">Logout</button>
      </p>
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
