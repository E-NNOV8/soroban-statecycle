"""Shared failure types for imported and module-executed entry points."""

class Inconclusive(Exception):
    def __init__(self, message, field=None, context=None):
        super().__init__(message)
        self.message = str(message)
        self.field = field
        self.context = context

    def __str__(self):
        parts = [self.message]
        if self.field:
            parts.append(f"(field: {self.field})")
        if self.context:
            parts.append(f"[context: {self.context}]")
        return " ".join(parts)


class Violation(Exception):
    pass
