<template>
  <div class="field">
    <b-field label="Password" :type="message ? 'is-danger' : ''">
      <b-input v-model="password" type="password" />
    </b-field>

    <b-field
      label="Password again"
      :type="password_again_message ? 'is-danger' : ''"
      :message="password_again_message"
    >
      <b-input v-model="password_again" type="password" />
    </b-field>
  </div>
</template>

<script>
export default {
  props: ["message"],
  data: () => ({
    password: "",
    password_again: "",
  }),
  computed: {
    passwords_same() {
      return this.password == this.password_again;
    },
    password_again_message() {
      return this.passwords_same ? "" : "Passwords don't match.";
    },
  },
  watch: {
    passwords_same: function() {
      this.$emit("input", this.passwords_same ? this.password : "");
    },
  },
};
</script>
