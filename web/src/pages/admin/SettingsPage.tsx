import { useEffect, useState } from 'react';
import { PageHeader, PanelCard } from '../../components/Page';
import { DataTable } from '../../components';
import { listSettings, type Setting } from '../../api/admin';
import { formatTime } from '../../utils/format';

export default function SettingsPage() {
  const [rows, setRows] = useState<Setting[]>([]);
  useEffect(() => { void listSettings().then(setRows); }, []);

  return (
    <div className="page-shell">
      <PageHeader title="系统设置" description="第一版系统设置为只读，后续按部署侧策略开放。" />
      <PanelCard title="设置项">
        <DataTable<Setting> rows={rows} columns={[
          { key: 'key', title: '键' },
          { key: 'value', title: '值' },
          { key: 'description', title: '说明' },
          { key: 'updated_at', title: '更新时间', render: row => formatTime(row.updated_at) },
        ]} />
      </PanelCard>
    </div>
  );
}