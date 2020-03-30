import hypothesis

hypothesis.settings.register_profile("dev", print_blob=True)
hypothesis.settings.load_profile("dev")
