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
    def __init__(self, message, code=None, details=None):
        super().__init__(message)
        self.message = str(message)
        self.code = code or 'INVARIANT_VIOLATION'
        self.details = details or {}

    def to_dict(self):
        return {
            'error_type': 'Violation',
            'code': self.code,
            'message': self.message,
            'details': self.details,
        }

    def __str__(self):
        if self.code and self.code != 'INVARIANT_VIOLATION':
            return f"[{self.code}] {self.message}"
        return self.message

