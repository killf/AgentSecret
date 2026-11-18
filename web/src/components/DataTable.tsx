import { Table } from 'antd';
import type { ColumnType } from 'antd/es/table';
import type { ReactNode } from 'react';

export interface Column<T> {
  key: string;
  title: string;
  render?: (item: T) => ReactNode;
  width?: number;
}

export function DataTable<T>({ columns, rows, pageSize = 20 }: { columns: Column<T>[]; rows: T[]; pageSize?: number }) {
  const tableColumns: ColumnType<T>[] = columns.map(column => ({
    key: column.key,
    title: column.title,
    width: column.width,
    dataIndex: column.key as never,
    render: column.render ? (_, record: T) => column.render?.(record) : undefined,
  }));

  return (
    <Table<T>
      columns={tableColumns}
      dataSource={rows}
      rowKey={record => String((record as { id?: number | string }).id ?? JSON.stringify(record))}
      size="small"
      tableLayout="fixed"
      scroll={{ x: 'max-content' }}
      pagination={{
        pageSize,
        showSizeChanger: false,
        showTotal: total => `共 ${total} 条`,
      }}
      locale={{ emptyText: '暂无数据' }}
    />
  );
}