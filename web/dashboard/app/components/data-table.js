// Data table: native table with an optional caption, column headers, rows, and an empty row.
// On narrow screens CSS turns each row into a card, reading the data-label attributes.

function emptyRow(span, text) {
  const tr = document.createElement('tr');
  tr.setAttribute('role', 'row');
  tr.className = 'data-table-empty';
  const td = document.createElement('td');
  td.setAttribute('role', 'cell');
  td.colSpan = span;
  td.textContent = text;
  tr.append(td);
  return tr;
}

/**
 * @param {{caption?: string, columns: Array<{key: string, label: string, mobileHead?: 'primary' | 'secondary'}>,
 *   rows?: Array<Array<unknown>>, emptyText?: string, numericColumns?: string[]}} options
 * @returns {HTMLTableElement}
 */
export function createDataTable({
  caption, columns, rows = [], emptyText = '', numericColumns = [],
} = {}) {
  if (!Array.isArray(columns) || columns.length === 0) {
    throw new TypeError('table columns are required');
  }
  for (const column of columns) {
    if (typeof column?.key !== 'string' || typeof column?.label !== 'string') {
      throw new TypeError('each column needs a string key and label');
    }
    if (column.mobileHead !== undefined && !['primary', 'secondary'].includes(column.mobileHead)) {
      throw new TypeError(`unknown mobile head: ${String(column.mobileHead)}`);
    }
  }
  const keys = new Set(columns.map((column) => column.key));
  for (const key of numericColumns) {
    if (!keys.has(key)) throw new TypeError(`unknown numeric column: ${String(key)}`);
  }
  const numeric = new Set(numericColumns);

  const table = document.createElement('table');
  table.className = 'data-table';
  table.setAttribute('role', 'table');
  if (caption) {
    const captionEl = document.createElement('caption');
    captionEl.className = 'data-table-caption';
    captionEl.textContent = caption;
    table.append(captionEl);
  }

  const headRow = document.createElement('tr');
  headRow.setAttribute('role', 'row');
  for (const column of columns) {
    const th = document.createElement('th');
    th.setAttribute('role', 'columnheader');
    th.scope = 'col';
    th.textContent = column.label;
    if (numeric.has(column.key)) th.className = 'data-table-num';
    headRow.append(th);
  }
  const thead = document.createElement('thead');
  thead.setAttribute('role', 'rowgroup');
  thead.append(headRow);

  const tbody = document.createElement('tbody');
  tbody.setAttribute('role', 'rowgroup');
  if (rows.length === 0) tbody.append(emptyRow(columns.length, emptyText));
  for (const values of rows) {
    const tr = document.createElement('tr');
    tr.setAttribute('role', 'row');
    columns.forEach((column, index) => {
      const td = document.createElement('td');
      td.setAttribute('role', 'cell');
      td.setAttribute('data-label', column.label);
      if (values[index] instanceof Node) {
        td.append(values[index]);
      } else {
        td.textContent = String(values[index] ?? '');
      }
      if (numeric.has(column.key)) td.className = 'data-table-num';
      if (column.mobileHead) td.classList.add(`data-table-head-${column.mobileHead}`);
      tr.append(td);
    });
    tbody.append(tr);
  }

  table.append(thead, tbody);
  return table;
}
