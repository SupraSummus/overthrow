import Vue from "vue";
import App from "./App.vue";

Vue.config.productionTip = false;

// import vue-panzoom
import panZoom from "vue-panzoom";

// install plugin
Vue.use(panZoom);

new Vue({
  render: h => h(App),
}).$mount("#app");
