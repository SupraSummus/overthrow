import store from "./store";

const api_port = 8000;
const api_url = `${window.location.protocol}//${window.location.hostname}:${api_port}/api/`;

const call_api = ({ method, path, payload }) => {
  const headers = { "Content-Type": "application/json" };
  if (store.state.auth_token) {
    headers["Authorization"] = "Token " + store.state.auth_token;
  }
  return fetch(api_url + path, {
    method,
    headers,
    body: JSON.stringify(payload),
  }).then(response => {
    // no content
    if (response.status == 204) return null;

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
