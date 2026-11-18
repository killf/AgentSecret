import { useEffect, useState } from 'react';
import { App as AntApp, Button, Drawer, Form, Input, InputNumber, Popconfirm, Select, Space, Tabs, Typography } from 'antd';
import { CheckOutlined, CloseOutlined, PlayCircleOutlined, PlusOutlined } from '@ant-design/icons';
import {
  addEnvironment,
  addProject,
  deleteEnvironment,
  deleteProject,
  listEnvironments,
  listProjects,
  updateEnvironment,
  updateProject,
  type Environment,
  type Project,
} from '../api/projects';
import {
  addScript,
  deleteScript,
  getScript,
  listScripts,
  reviewScript,
  runScript,
  updateScript,
  type ScriptItem,
} from '../api/scripts';
import { addVariable, deleteVariable, listVariables, updateVariable, type VariableItem } from '../api/variables';
import { PageHeader, PanelCard } from '../components/Page';
import { Badge, DataTable } from '../components';
import { formatTime } from '../utils/format';

export default function ProjectsPage() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [environments, setEnvironments] = useState<Environment[]>([]);
  const [scripts, setScripts] = useState<ScriptItem[]>([]);
  const [variables, setVariables] = useState<VariableItem[]>([]);
  const [projectId, setProjectId] = useState<number | null>(null);
  const [projectName, setProjectName] = useState('');
  const [environmentName, setEnvironmentName] = useState('dev');
  const [scriptName, setScriptName] = useState('');
  const [scriptType, setScriptType] = useState<'bash' | 'python'>('bash');
  const [scriptContent, setScriptContent] = useState('');
  const [scriptParams, setScriptParams] = useState('');
  const [scriptCredentialActions, setScriptCredentialActions] = useState('');
  const [editingScript, setEditingScript] = useState<{ id: number; revision: number } | null>(null);
  const [environmentId, setEnvironmentId] = useState<number | ''>('');
  const [sandboxId, setSandboxId] = useState<number | ''>('');
  const [variableForm, setVariableForm] = useState({ environment_id: '', key: '', source_env: '', description: '' });
  const [editingProject, setEditingProject] = useState<Project | null>(null);
  const [editingEnvironment, setEditingEnvironment] = useState<Environment | null>(null);
  const { message } = AntApp.useApp();

  async function refreshVariables(currentProjectId: number, currentEnvironmentId: number) {
    setVariables(await listVariables(currentProjectId, currentEnvironmentId));
  }

  async function refreshScripts(currentProjectId: number) {
    setScripts(await listScripts(currentProjectId));
  }

  async function refresh() {
    const rows = await listProjects();
    setProjects(rows);
    const current = projectId ?? rows[0]?.id ?? null;
    setProjectId(current);
    if (!current) return;

    const environmentRows = await listEnvironments(current);
    setEnvironments(environmentRows);
    if (environmentRows[0]) await refreshVariables(current, environmentRows[0].id);
    await refreshScripts(current);
  }

  useEffect(() => { void refresh(); }, []);

  async function refreshEnvironmentData(currentProjectId: number) {
    const environmentRows = await listEnvironments(currentProjectId);
    setEnvironments(environmentRows);
    if (environmentRows[0]) await refreshVariables(currentProjectId, environmentRows[0].id);
  }

  async function createProject() {
    await addProject({ name: projectName });
    setProjectName('');
    message.success('项目已创建');
    await refresh();
  }

  async function createEnvironment() {
    if (!projectId) return;
    await addEnvironment({ project_id: projectId, name: environmentName });
    message.success('环境已创建');
    await refreshEnvironmentData(projectId);
  }

  async function saveProject() {
    if (!editingProject) return;
    await updateProject({ id: editingProject.id, name: editingProject.name });
    setEditingProject(null);
    await refresh();
  }

  async function saveEnvironment() {
    if (!editingEnvironment) return;
    await updateEnvironment({ id: editingEnvironment.id, name: editingEnvironment.name });
    setEditingEnvironment(null);
    await refresh();
  }

  async function loadVariables(environmentId: number) {
    if (!projectId) return;
    setVariableForm(current => ({ ...current, environment_id: String(environmentId) }));
    await refreshVariables(projectId, environmentId);
  }

  async function toggleVariable(row: VariableItem) {
    await updateVariable({ id: row.id, disabled_at: row.disabled_at ? null : new Date().toISOString() });
    if (projectId) {
      await refreshVariables(projectId, Number(variableForm.environment_id || environments[0]?.id || 0));
      await refreshScripts(projectId);
    }
  }

  async function removeVariable(row: VariableItem) {
    await deleteVariable(row.id);
    if (projectId) {
      await refreshVariables(projectId, Number(variableForm.environment_id || environments[0]?.id || 0));
      await refreshScripts(projectId);
    }
  }

  async function createVariable() {
    if (!projectId) return;
    const currentEnvironmentId = Number(variableForm.environment_id || environments[0]?.id || 0);
    await addVariable({
      project_id: projectId,
      environment_id: currentEnvironmentId,
      key: variableForm.key,
      source_env: variableForm.source_env,
      description: variableForm.description || undefined,
    });
    setVariableForm({ environment_id: String(currentEnvironmentId), key: '', source_env: '', description: '' });
    message.success('凭据绑定已创建');
    await refreshVariables(projectId, currentEnvironmentId);
  }

  function parseOptionalJson(source: string, field: string): unknown | undefined {
    if (!source.trim()) return undefined;
    try {
      return JSON.parse(source);
    } catch {
      throw new Error(`${field} 不是合法 JSON`);
    }
  }

  async function editScript(row: ScriptItem) {
    const detail = await getScript(row.id);
    setEditingScript({ id: row.id, revision: row.revision });
    setScriptName(detail.name);
    setScriptType(detail.type);
    setScriptContent(detail.content ?? '');
    setEnvironmentId(detail.environment_id ?? '');
    setSandboxId(detail.sandbox_id ?? '');
    setScriptParams(detail.params === null ? '' : JSON.stringify(detail.params, null, 2));
    setScriptCredentialActions(
      detail.credential_actions === null || detail.credential_actions === undefined
        ? ''
        : JSON.stringify(detail.credential_actions, null, 2),
    );
  }

  async function saveScript() {
    if (!projectId) return;
    try {
      const params = parseOptionalJson(scriptParams, '参数定义');
      const credentialActions = parseOptionalJson(scriptCredentialActions, '凭据操作');
      const body = {
        project_id: projectId,
        name: scriptName,
        type: scriptType,
        content: scriptContent,
        environment_id: environmentId === '' ? null : environmentId,
        sandbox_id: sandboxId === '' ? null : sandboxId,
        ...(params === undefined ? {} : { params }),
        ...(credentialActions === undefined ? {} : { credential_actions: credentialActions }),
      };
      const wasEditing = editingScript !== null;
      if (editingScript) await updateScript({ id: editingScript.id, revision: editingScript.revision, ...body });
      else await addScript(body);

      setEditingScript(null);
      setScriptName('');
      setScriptContent('');
      setScriptParams('');
      setScriptCredentialActions('');
      message.success(wasEditing ? '脚本已更新，等待审核' : '脚本已创建，等待审核');
      await refreshScripts(projectId);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '保存失败');
    }
  }

  const currentEnvironmentId = Number(variableForm.environment_id || environments[0]?.id || 0);

  return (
    <div className="page-shell">
      <PageHeader title="项目与脚本" description="管理项目隔离范围、执行环境、凭据绑定和审核脚本。" />

      <PanelCard title="当前工作区">
        <Space wrap>
          <Select
            value={projectId ?? undefined}
            onChange={async value => {
              setProjectId(value);
              setVariableForm({ environment_id: '', key: '', source_env: '', description: '' });
              const environmentRows = await listEnvironments(value);
              setEnvironments(environmentRows);
              if (environmentRows[0]) await refreshVariables(value, environmentRows[0].id);
              await refreshScripts(value);
            }}
            placeholder="选择项目"
            style={{ width: 280 }}
            options={projects.map(project => ({ value: project.id, label: project.name }))}
          />
          <Input value={projectName} onChange={event => setProjectName(event.target.value)} placeholder="新项目名称" style={{ width: 220 }} />
          <Button type="primary" icon={<PlusOutlined />} onClick={() => void createProject()}>创建项目</Button>
          <Select value={environmentName} onChange={setEnvironmentName} style={{ width: 120 }} options={[
            { value: 'dev', label: 'dev' },
            { value: 'test', label: 'test' },
            { value: 'prod', label: 'prod' },
          ]} />
          <Button disabled={!projectId} onClick={() => void createEnvironment()}>创建环境</Button>
        </Space>
      </PanelCard>

      <Tabs
        defaultActiveKey="scripts"
        items={[
          { key: 'scripts', label: '脚本', children: (
            <>
              <PanelCard title={editingScript ? `编辑脚本 #${editingScript.id}` : '新增脚本'}>
                <Form layout="vertical">
                  <div className="form-grid">
                    <Form.Item label="名称"><Input value={scriptName} onChange={event => setScriptName(event.target.value)} /></Form.Item>
                    <Form.Item label="类型">
                      <Select value={scriptType} onChange={setScriptType} options={[
                        { value: 'bash', label: 'Bash' },
                        { value: 'python', label: 'Python' },
                      ]} />
                    </Form.Item>
                    <Form.Item label="环境">
                      <Select allowClear value={environmentId === '' ? undefined : environmentId} onChange={value => setEnvironmentId(value ?? '')} options={environments.map(environment => ({ value: environment.id, label: environment.name }))} />
                    </Form.Item>
                    <Form.Item label="沙盒 ID"><InputNumber min={1} value={sandboxId === '' ? undefined : sandboxId} onChange={value => setSandboxId(value ?? '')} style={{ width: '100%' }} /></Form.Item>
                    <Form.Item label="脚本内容" className="full"><Input.TextArea rows={6} value={scriptContent} onChange={event => setScriptContent(event.target.value)} /></Form.Item>
                    <Form.Item label="参数定义 JSON" className="full"><Input.TextArea rows={3} value={scriptParams} onChange={event => setScriptParams(event.target.value)} placeholder='可空，例如 {"release":{"type":"string","required":true}}' /></Form.Item>
                    <Form.Item label="凭据操作 JSON" className="full"><Input.TextArea rows={3} value={scriptCredentialActions} onChange={event => setScriptCredentialActions(event.target.value)} placeholder='可空，例如 {"actions":[{"action_id":"release.status","binding_id":1}]}' /></Form.Item>
                    <Form.Item label=" ">
                      <Space>
                        <Button type="primary" onClick={() => void saveScript()}>{editingScript ? '保存脚本' : '创建脚本'}</Button>
                        {editingScript && <Button onClick={() => setEditingScript(null)}>取消编辑</Button>}
                      </Space>
                    </Form.Item>
                  </div>
                </Form>
              </PanelCard>

              <PanelCard title="脚本列表">
                <DataTable<ScriptItem> rows={scripts} columns={[
                  { key: 'name', title: '名称' },
                  { key: 'type', title: '类型' },
                  { key: 'revision', title: '版本' },
                  { key: 'review_status', title: '状态', render: row => <Badge tone={row.review_status === 'approved' ? 'success' : row.review_status === 'rejected' ? 'danger' : 'running'}>{row.review_status}</Badge> },
                  { key: 'updated_at', title: '更新时间', render: row => formatTime(row.updated_at) },
                  { key: 'actions', title: '操作', render: row => (
                    <Space size={4}>
                      <Button size="small" type="text" onClick={() => void editScript(row)}>编辑</Button>
                      <Button size="small" type="text" icon={<CheckOutlined />} disabled={row.review_status !== 'pending'} onClick={() => projectId && void reviewScript(row.id, row.revision, 'approve').then(() => refreshScripts(projectId))}>通过</Button>
                      <Button size="small" type="text" icon={<CloseOutlined />} disabled={row.review_status !== 'pending'} onClick={() => projectId && void reviewScript(row.id, row.revision, 'reject').then(() => refreshScripts(projectId))}>拒绝</Button>
                      <Button size="small" type="text" icon={<PlayCircleOutlined />} disabled={row.review_status !== 'approved'} onClick={() => void runScript(row.id).then(result => message.success(`执行 ID：${result.execution_id}`))}>执行</Button>
                      <Popconfirm title="删除脚本？" onConfirm={() => projectId && void deleteScript(row.id).then(() => refreshScripts(projectId))}>
                        <Button size="small" type="text" danger>删除</Button>
                      </Popconfirm>
                    </Space>
                  ) },
                ]} />
              </PanelCard>
            </>
          ) },
          { key: 'variables', label: '凭据绑定', children: (
            <>
              <PanelCard title="新增凭据绑定">
                <Form layout="vertical">
                  <div className="form-grid">
                    <Form.Item label="环境">
                      <Select value={currentEnvironmentId || undefined} onChange={value => void loadVariables(value)} options={environments.map(environment => ({ value: environment.id, label: environment.name }))} />
                    </Form.Item>
                    <Form.Item label="变量别名"><Input value={variableForm.key} onChange={event => setVariableForm({ ...variableForm, key: event.target.value })} /></Form.Item>
                    <Form.Item label="来源环境变量"><Input value={variableForm.source_env} onChange={event => setVariableForm({ ...variableForm, source_env: event.target.value })} /></Form.Item>
                    <Form.Item label="说明"><Input value={variableForm.description} onChange={event => setVariableForm({ ...variableForm, description: event.target.value })} /></Form.Item>
                    <Form.Item label=" "><Button type="primary" onClick={() => void createVariable()}>新增绑定</Button></Form.Item>
                  </div>
                </Form>
              </PanelCard>

              <PanelCard title="绑定列表" extra={<Typography.Text type="secondary">仅显示映射，不显示密钥值</Typography.Text>}>
                <DataTable<VariableItem> rows={variables} columns={[
                  { key: 'key', title: '别名' },
                  { key: 'source_env', title: '来源环境变量' },
                  { key: 'description', title: '说明', render: row => row.description || '—' },
                  { key: 'disabled_at', title: '状态', render: row => <Badge tone={row.disabled_at ? 'muted' : 'success'}>{row.disabled_at ? '禁用' : '启用'}</Badge> },
                  { key: 'actions', title: '操作', render: row => (
                    <Space size={4}>
                      <Button size="small" type="text" onClick={() => void toggleVariable(row)}>{row.disabled_at ? '启用' : '禁用'}</Button>
                      <Popconfirm title={`删除凭据绑定 ${row.key}？`} onConfirm={() => void removeVariable(row)}>
                        <Button size="small" type="text" danger>删除</Button>
                      </Popconfirm>
                    </Space>
                  ) },
                ]} />
              </PanelCard>
            </>
          ) },
          { key: 'environments', label: '环境', children: (
            <PanelCard title="环境列表">
              <DataTable<Environment> rows={environments} columns={[
                { key: 'name', title: '名称' },
                { key: 'description', title: '描述', render: row => row.description || '—' },
                { key: 'created_at', title: '创建时间', render: row => formatTime(row.created_at) },
                { key: 'actions', title: '操作', render: row => (
                  <Space size={4}>
                    <Button size="small" type="text" onClick={() => setEditingEnvironment(structuredClone(row))}>编辑</Button>
                    <Popconfirm title={`删除环境 ${row.name}？`} onConfirm={() => void deleteEnvironment(row.id).then(refresh)}>
                      <Button size="small" type="text" danger>删除</Button>
                    </Popconfirm>
                  </Space>
                ) },
              ]} />
            </PanelCard>
          ) },
          { key: 'projects', label: '项目', children: (
            <PanelCard title="项目列表">
              <DataTable<Project> rows={projects} columns={[
                { key: 'name', title: '名称' },
                { key: 'description', title: '描述', render: row => row.description || '—' },
                { key: 'repos', title: '仓库', render: row => row.repos.join(', ') || '—' },
                { key: 'created_at', title: '创建时间', render: row => formatTime(row.created_at) },
                { key: 'actions', title: '操作', render: row => (
                  <Space size={4}>
                    <Button size="small" type="text" onClick={() => setEditingProject(structuredClone(row))}>编辑</Button>
                    <Popconfirm title={`删除项目 ${row.name}？`} onConfirm={() => void deleteProject(row.id).then(refresh)}>
                      <Button size="small" type="text" danger>删除</Button>
                    </Popconfirm>
                  </Space>
                ) },
              ]} />
            </PanelCard>
          ) },
        ]}
      />

      <Drawer title="编辑项目" open={editingProject !== null} onClose={() => setEditingProject(null)} width={460}>
        {editingProject && (
          <Form layout="vertical">
            <Form.Item label="项目名称"><Input value={editingProject.name} onChange={event => setEditingProject({ ...editingProject, name: event.target.value })} /></Form.Item>
            <Button type="primary" onClick={() => void saveProject()}>保存</Button>
          </Form>
        )}
      </Drawer>

      <Drawer title="编辑环境" open={editingEnvironment !== null} onClose={() => setEditingEnvironment(null)} width={460}>
        {editingEnvironment && (
          <Form layout="vertical">
            <Form.Item label="环境名称"><Input value={editingEnvironment.name} onChange={event => setEditingEnvironment({ ...editingEnvironment, name: event.target.value })} /></Form.Item>
            <Button type="primary" onClick={() => void saveEnvironment()}>保存</Button>
          </Form>
        )}
      </Drawer>
    </div>
  );
}