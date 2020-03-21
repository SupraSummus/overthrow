<template>
  <section class="section">
    <div class="container">
      <form @submit.prevent="login">
        <b-field
          v-for="(field, name) in fields"
          :key="name"
          v-bind="field"
          :type="field.message ? 'is-danger' : ''"
        >
          <b-input v-model="field.value" v-bind="field.input_options" />
        </b-field>

        <b-button
          type="is-primary"
          native-type="submit"
          v-bind:loading="processing"
        >
          Log in
        </b-button>

        <p class="has-text-danger">{{ message }}</p>
      </form>
    </div>
  </section>
</template>

<script>
export default {
  data() {
    return {
      fields: {
        username: {
          label: "Username",
          message: "",
          value: "",
          input_options: { maxlength: 30 },
        },
        password: {
          label: "Password",
          message: "",
          value: "",
          input_options: { type: "password", "password-reveal": true },
        },
      },
      processing: false,
      message: "",
    };
  },
  methods: {
    login: function() {
      this.processing = true;
      this.$store
        .dispatch("log_in", {
          username: this.fields.username.value,
          password: this.fields.password.value,
        })
        .then(() =>
          this.$router.push({ name: "game", params: { id: "default" } }),
        )
        .catch(e => {
          // clear status
          this.processing = false;
          this.message = "";
          for (let name in this.fields) {
            this.fields[name].message = "";
          }

          // probably server error
          // instanceof doesn't work because of some babel magic
          if (e.name != "APIClientError") {
            this.message = "Oops, something went wrong.";
            throw e;
          }

          // set messages
          if ("non_field_errors" in e.response) {
            this.message = e.response["non_field_errors"].join(", ");
          }
          for (let name in this.fields) {
            const field = this.fields[name];
            if (name in e.response) field.message = e.response[name].join(", ");
          }
        });
    },
  },
};
</script>
