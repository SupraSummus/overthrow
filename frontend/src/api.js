import store from "./store";

const call_api = ({ method, path, payload }) => {
  const headers = { "Content-Type": "application/json" };
  if (store.state.auth.token) {
    headers["Authorization"] = "Token " + store.state.auth.token;
  }
  return fetch("http://localhost:8000/api/" + path, {
    method,
    headers,
    body: JSON.stringify(payload),
  }).then(response => {
    if (response.ok) {
      return response.json();
    } else if (response.status >= 400 && response.status < 500) {
      throw response.json();
    } else {
      console.error(response);
      throw "unexpected response";
    }
  });
};

export default call_api;
