import Vue from "vue";
import panZoom from "vue-panzoom";

import App from "./App.vue";
import store from "./store";

Vue.config.productionTip = false;

// install plugins
Vue.use(panZoom);

new Vue({
  render: h => h(App),
  store,
}).$mount("#app");
