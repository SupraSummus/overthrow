<template>
  <section class="section">
    <div class="container">
      <form @submit.prevent="sign_up">
        <b-field
          label="Username"
          :type="fields.username.message ? 'is-danger' : ''"
          :message="fields.username.message"
        >
          <b-input v-model="fields.username.value" maxlength="30" />
        </b-field>

        <double-password-field
          :message="fields.password.message"
          v-model="fields.password.value"
        />

        <div class="buttons">
          <b-button
            type="is-primary"
            native-type="submit"
            :loading="processing"
            :disabled="!fields.password.value"
          >
            Sign up
          </b-button>
        </div>

        <p class="has-text-danger">{{ message }}</p>
      </form>
    </div>
  </section>
</template>

<script>
import DoublePasswordField from "@/components/double_password_field.vue";
import call_api from "@/api";
import { update_form_messages } from "@/forms";

export default {
  components: { DoublePasswordField },
  data() {
    return {
      fields: {
        username: {
          message: "",
          value: "",
        },
        password: {
          message: "",
          value: "",
        },
      },
      processing: false,
      message: "",
    };
  },
  methods: {
    sign_up: function() {
      this.processing = true;
      const create_user = call_api({
        method: "POST",
        path: "register/",
        payload: {
          username: this.fields.username.value,
          password: this.fields.password.value,
        },
      });
      create_user.catch(e => update_form_messages(this, e));
      create_user
        .then(() =>
          this.$store.dispatch("log_in", {
            username: this.fields.username.value,
            password: this.fields.password.value,
          }),
        )
        .then(() =>
          this.$router.push({ name: "game", params: { id: "default" } }),
        );
    },
  },
};
</script>
