import Vue from "vue";
import panZoom from "vue-panzoom";

import App from "./App.vue";
import store from "./store";
import router from "./router";
import vuetify from "./plugins/vuetify";

Vue.config.productionTip = false;

// install plugins
Vue.use(panZoom);

new Vue({
  render: h => h(App),
  store,
  vuetify,
  router,
}).$mount("#app");
