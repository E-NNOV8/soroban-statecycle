"""Shared failure types for imported and module-executed entry points."""

class Inconclusive(Exception):
    pass


class Violation(Exception):
    pass
