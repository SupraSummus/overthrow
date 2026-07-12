export function update_form_messages(form, error) {
  // clear status
  form.processing = false;
  form.message = "";
  for (let name in form.fields) {
    form.fields[name].message = "";
  }

  // probably server error
  // instanceof doesn't work because of some babel magic
  if (error.name != "APIClientError") {
    form.message = "Oops, something went wrong.";
    throw error;
  }

  // set messages
  if ("non_field_errors" in error.response) {
    form.message = error.response["non_field_errors"].join(", ");
  }
  for (let name in form.fields) {
    const field = form.fields[name];
    if (name in error.response) field.message = error.response[name].join(", ");
  }
}
