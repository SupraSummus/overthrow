"""
This module provides very simple Django middleware that sleeps on every request.
This is useful when you want to simulate slow response times (as might be
encountered, say, on a cell network).
To use, add this middleware, and add a value for SLEEP_TIME to your settings.
Possible future feature: Look for an X-Django-Sleep header on each request,
to let the client specify per-request sleep time.

Based on
https://gist.github.com/josharian/1453629
"""

import time

import django.conf
import django.core.exceptions


class DelayMiddleware:

    def __init__(self, get_response):
        self.get_response = get_response
        self.sleep_time = getattr(django.conf.settings, "SLEEP_TIME", 0)
        if not isinstance(self.sleep_time, (int, float)) or self.sleep_time <= 0:
            raise django.core.exceptions.MiddlewareNotUsed

    def __call__(self, request):
        time.sleep(self.sleep_time)
        return self.get_response(request)
