<template>
  <div>
    <form @submit.prevent="login">
      <h1>Sign in</h1>
      <label>
        Username
        <input required v-model="username" type="username" placeholder="Name" />
      </label>
      <label>
        Password
        <input
          required
          v-model="password"
          type="password"
          placeholder="Password"
        />
      </label>
      <p>{{ message }}</p>
      <p></p>
      <hr />
      <button type="submit">Login</button>
    </form>
  </div>
</template>

<script>
export default {
  data() {
    return {
      username: "",
      password: "",
      message: "",
    };
  },
  methods: {
    login: function() {
      this.message = "logging in ...";
      this.$store
        .dispatch("login", { username: this.username, password: this.password })
        .catch(() => {
          this.message = "failed to login";
        });
    },
  },
};
</script>
