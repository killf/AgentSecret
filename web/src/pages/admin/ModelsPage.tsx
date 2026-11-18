import { useEffect, useState } from 'react';
import { App as AntApp, Button, Col, Drawer, Form, Input, InputNumber, Popconfirm, Row, Select, Space, Switch } from 'antd';
import { ApiOutlined, PlusOutlined, ReloadOutlined } from '@ant-design/icons';
import { PageHeader, PanelCard } from '../../components/Page';
import { Badge, DataTable } from '../../components';
import {
  addModel,
  addModelProvider,
  deleteModel,
  deleteModelProvider,
  listModelProviders,
  listModels,
  listProviders,
  testModelProvider,
  updateModel,
  updateModelProvider,
  type ModelItem,
  type ModelProviderRelation,
  type ProviderInfo,
} from '../../api/admin';
import { formatTime } from '../../utils/format';

interface RelationForm {
  model_id: number;
  provider_id: number;
  model_name: string;
  max_output_tokens: number;
  max_context_tokens: number;
}

export default function ModelsPage() {
  const [models, setModels] = useState<ModelItem[]>([]);
  const [relations, setRelations] = useState<ModelProviderRelation[]>([]);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [editingRelation, setEditingRelation] = useState<ModelProviderRelation | null>(null);
  const [editingModel, setEditingModel] = useState<ModelItem | null>(null);
  const [message, setMessage] = useState('');
  const [relationForm] = Form.useForm<RelationForm>();
  const { message: apiMessage } = AntApp.useApp();

  async function refresh() {
    const [modelRows, relationRows, providerRows] = await Promise.all([
      listModels(), listModelProviders(), listProviders(),
    ]);
    setModels(modelRows);
    setRelations(relationRows);
    setProviders(providerRows);
  }

  async function createModel(values: { name: string }) {
    await addModel(values.name);
    apiMessage.success('模型已创建');
    await refresh();
  }

  async function saveModel() {
    if (!editingModel) return;
    await updateModel({ id: editingModel.id, name: editingModel.name });
    setEditingModel(null);
    await refresh();
  }

  async function createRelation(values: RelationForm) {
    const created = await addModelProvider({ ...values, enabled: true });
    await testRelation(created.id);
    relationForm.resetFields();
    await refresh();
  }

  async function testRelation(id: number) {
    const result = await testModelProvider(id).catch(() => null);
    setMessage(result ? `连通性：${result.ok ? '成功' : '失败'}` : '连通性测试失败');
  }

  async function saveRelation() {
    if (!editingRelation) return;

    await updateModelProvider({
      id: editingRelation.id,
      model_name: editingRelation.model_name,
      max_output_tokens: editingRelation.max_output_tokens,
      max_context_tokens: editingRelation.max_context_tokens,
      enabled: editingRelation.enabled,
    });
    setEditingRelation(null);
    apiMessage.success('供应商关系已更新');
    await refresh();
  }

  useEffect(() => { void refresh(); }, []);

  return (
    <div className="page-shell">
      <PageHeader
        title="模型管理"
        description="维护模型、上游供应商关系和连通性状态。"
        extra={<Button icon={<ReloadOutlined />} onClick={() => void refresh()}>刷新</Button>}
      />

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={8}>
          <PanelCard title="新增模型">
          <Form layout="vertical" onFinish={createModel} requiredMark={false}>
            <Form.Item label="模型名称" name="name" rules={[{ required: true, message: '请输入模型名称' }]}>
              <Input placeholder="例如 glm-4.7" />
            </Form.Item>
            <Button type="primary" htmlType="submit" icon={<PlusOutlined />}>创建</Button>
          </Form>
          </PanelCard>
        </Col>
        <Col xs={24} lg={16}>
          <PanelCard title="模型列表">
          <DataTable<ModelItem> rows={models} columns={[
            { key: 'name', title: '名称' },
            { key: 'status', title: '状态', render: row => <Badge tone={row.disabled_at ? 'muted' : 'success'}>{row.disabled_at ? '禁用' : '启用'}</Badge> },
            { key: 'created_at', title: '创建时间', render: row => formatTime(row.created_at) },
            { key: 'actions', title: '操作', render: row => (
              <Space size={4}>
                <Button size="small" type="text" onClick={() => setEditingModel(structuredClone(row))}>编辑</Button>
                <Button size="small" type="text" onClick={() => void updateModel({ id: row.id, disabled_at: row.disabled_at ? null : new Date().toISOString() }).then(refresh)}>{row.disabled_at ? '启用' : '禁用'}</Button>
                <Popconfirm title={`删除模型 ${row.name}？`} onConfirm={() => void deleteModel(row.id).then(refresh)}>
                  <Button size="small" type="text" danger>删除</Button>
                </Popconfirm>
              </Space>
            ) },
            ]} />
          </PanelCard>
        </Col>
      </Row>

      <div className="section-title">模型-供应商关系</div>
      <PanelCard title="新增关系">
        <Form<RelationForm> form={relationForm} layout="vertical" onFinish={createRelation} requiredMark={false} initialValues={{ max_output_tokens: 4096, max_context_tokens: 200000 }}>
          <div className="form-grid">
            <Form.Item label="模型" name="model_id" rules={[{ required: true, message: '请选择模型' }]}>
              <Select options={models.map(model => ({ value: model.id, label: `${model.name} (#${model.id})` }))} />
            </Form.Item>
            <Form.Item label="供应商" name="provider_id" rules={[{ required: true, message: '请选择供应商' }]}>
              <Select options={providers.map(provider => ({ value: provider.id, label: `${provider.name} (#${provider.id})` }))} />
            </Form.Item>
            <Form.Item label="上游模型名" name="model_name" rules={[{ required: true, message: '请输入上游模型名' }]}>
              <Input />
            </Form.Item>
            <Form.Item label="最长输出" name="max_output_tokens">
              <InputNumber min={1} style={{ width: '100%' }} />
            </Form.Item>
            <Form.Item label="最长上下文" name="max_context_tokens">
              <InputNumber min={1} style={{ width: '100%' }} />
            </Form.Item>
            <Form.Item label=" ">
              <Button type="primary" htmlType="submit">创建并测试</Button>
            </Form.Item>
          </div>
        </Form>
      </PanelCard>

      {message && <PanelCard title="连通性结果">{message}</PanelCard>}

      <PanelCard title="关系列表">
        <DataTable<ModelProviderRelation> rows={relations} columns={[
          { key: 'model_id', title: '模型 ID' },
          { key: 'provider_id', title: '供应商 ID' },
          { key: 'model_name', title: '上游模型' },
          { key: 'max_output_tokens', title: '最长输出' },
          { key: 'max_context_tokens', title: '最长上下文' },
          { key: 'enabled', title: '状态', render: row => <Badge tone={row.enabled ? 'success' : 'muted'}>{row.enabled ? '启用' : '禁用'}</Badge> },
          { key: 'actions', title: '操作', render: row => (
            <Space size={4}>
              <Button size="small" type="text" icon={<ApiOutlined />} onClick={() => void testRelation(row.id)}>测试</Button>
              <Button size="small" type="text" onClick={() => setEditingRelation(structuredClone(row))}>编辑</Button>
              <Button size="small" type="text" onClick={() => void updateModelProvider({ id: row.id, enabled: !row.enabled }).then(refresh)}>{row.enabled ? '禁用' : '启用'}</Button>
              <Popconfirm title={`删除关系 #${row.id}？`} onConfirm={() => void deleteModelProvider(row.id).then(refresh)}>
                <Button size="small" type="text" danger>删除</Button>
              </Popconfirm>
            </Space>
          ) },
        ]} />
      </PanelCard>

      <Drawer title="编辑模型" open={editingModel !== null} onClose={() => setEditingModel(null)} width={480}>
        {editingModel && (
          <Form layout="vertical">
            <Form.Item label="模型名称">
              <Input value={editingModel.name} onChange={event => setEditingModel({ ...editingModel, name: event.target.value })} />
            </Form.Item>
            <Button type="primary" onClick={() => void saveModel()}>保存</Button>
          </Form>
        )}
      </Drawer>

      <Drawer title={`编辑关系 #${editingRelation?.id ?? ''}`} open={editingRelation !== null} onClose={() => setEditingRelation(null)} width={520}>
        {editingRelation && (
          <Form layout="vertical">
            <Form.Item label="上游模型名">
              <Input value={editingRelation.model_name} onChange={event => setEditingRelation({ ...editingRelation, model_name: event.target.value })} />
            </Form.Item>
            <Form.Item label="最长输出">
              <InputNumber min={1} value={editingRelation.max_output_tokens} onChange={value => setEditingRelation({ ...editingRelation, max_output_tokens: value ?? 1 })} style={{ width: '100%' }} />
            </Form.Item>
            <Form.Item label="最长上下文">
              <InputNumber min={1} value={editingRelation.max_context_tokens} onChange={value => setEditingRelation({ ...editingRelation, max_context_tokens: value ?? 1 })} style={{ width: '100%' }} />
            </Form.Item>
            <Form.Item label="启用">
              <Switch checked={editingRelation.enabled} onChange={checked => setEditingRelation({ ...editingRelation, enabled: checked })} />
            </Form.Item>
            <Button type="primary" onClick={() => void saveRelation()}>保存</Button>
          </Form>
        )}
      </Drawer>
    </div>
  );
}
