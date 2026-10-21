"""Aides communes aux archétypes cap : recherche de la densité maximale qui passe le QA."""
import math

import qa
from content_ext_cap import GROUPS  # noqa: F401  (réexporté pour les archétypes)


def densest(mk, fmt, ns):
    """Premier résultat propre en essayant n décroissant ; sinon le dernier (le QA du générateur le rejettera)."""
    last = None
    for n in ns:
        w = mk(n)
        if w is None:
            continue
        last = w
        if not qa.problems(w, fmt):
            return w
    return last


def table_dims(fmt, n, row_min, col_min, extra=0):
    """(colonnes, lignes) : moins de colonnes possible tout en gardant des lignes >= row_min."""
    z = fmt.zone
    for cols in range(1, 7):
        if z[2] / cols < col_min:
            break
        rows = math.ceil(n / cols) + extra
        if z[3] / rows >= row_min:
            return cols, rows
    return None
