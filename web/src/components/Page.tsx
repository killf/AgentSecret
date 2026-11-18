import type { ReactNode } from 'react';
import { Card, Flex, Typography } from 'antd';

export function PageHeader({
  title,
  description,
  extra,
}: {
  title: string;
  description?: string;
  extra?: ReactNode;
}) {
  return (
    <Flex className="page-header" align="flex-start" justify="space-between" gap={16}>
      <div>
        <Typography.Title level={3}>{title}</Typography.Title>
        {description && <Typography.Paragraph type="secondary">{description}</Typography.Paragraph>}
      </div>
      {extra && <Flex gap={8}>{extra}</Flex>}
    </Flex>
  );
}

export function PanelCard({
  title,
  extra,
  children,
  className,
}: {
  title?: ReactNode;
  extra?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <Card className={className} title={title} extra={extra} variant="borderless" styles={{ body: { paddingTop: title ? 12 : 0 } }}>
      {children}
    </Card>
  );
}