import { useEffect, useRef, useState } from 'react'
import { Button, Empty, Input, Layout, List, Popconfirm, Tag, Typography, message } from 'antd'
import { DeleteOutlined, PlusOutlined } from '@ant-design/icons'
import dayjs from 'dayjs'
import api from './api.js'

const { Sider, Content } = Layout
const { Text } = Typography

export default function App() {
  const [notes, setNotes] = useState([])
  const [activeId, setActiveId] = useState(null)
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const searchRef = useRef('')

  const load = async (q = searchRef.current) => {
    setLoading(true)
    try {
      const { data } = await api.get('/api/notes', { params: q ? { query: q } : {} })
      setNotes(data)
      setActiveId((cur) => (data.some((n) => n.id === cur) ? cur : (data[0]?.id ?? null)))
    } catch (e) {
      message.error('加载失败：' + (e.response?.data ?? e.message))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    load()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const active = notes.find((n) => n.id === activeId) ?? null

  const createNote = async () => {
    try {
      const { data } = await api.post('/api/notes', { title: '新建记录', content: '' })
      message.success('已创建')
      await load()
      setActiveId(data.id)
    } catch (e) {
      message.error('创建失败：' + (e.response?.data ?? e.message))
    }
  }

  // 编辑后 600ms 自动保存
  const saveTimer = useRef(null)
  const onEdit = (patch) => {
    if (!active) return
    const next = { ...active, ...patch }
    setNotes((prev) => prev.map((n) => (n.id === active.id ? next : n)))
    clearTimeout(saveTimer.current)
    saveTimer.current = setTimeout(() => saveNote(next), 600)
  }

  const saveNote = async (note) => {
    setSaving(true)
    try {
      await api.put(`/api/notes/${note.id}`, { title: note.title, content: note.content })
      await load() // 刷新列表排序与“更新于”时间
    } catch (e) {
      message.error('保存失败：' + (e.response?.data ?? e.message))
    } finally {
      setSaving(false)
    }
  }

  const removeNote = async (id) => {
    try {
      await api.delete(`/api/notes/${id}`)
      message.success('已删除')
      if (activeId === id) setActiveId(null)
      await load()
    } catch (e) {
      message.error('删除失败：' + (e.response?.data ?? e.message))
    }
  }

  return (
    <Layout className="app">
      <Sider width={320} theme="light" className="sider">
        <div className="sider-header">
          <Input.Search
            placeholder="搜索记录…"
            allowClear
            onSearch={(v) => {
              searchRef.current = v
              load(v)
            }}
          />
          <Button type="primary" icon={<PlusOutlined />} onClick={createNote}>
            新建
          </Button>
        </div>
        <List
          className="note-list"
          loading={loading}
          dataSource={notes}
          locale={{ emptyText: <Empty description="暂无记录" /> }}
          renderItem={(n) => (
            <List.Item
              className={n.id === activeId ? 'note-item active' : 'note-item'}
              onClick={() => setActiveId(n.id)}
            >
              <div className="note-item-body">
                <div className="note-item-title">{n.title || '未命名'}</div>
                <div className="note-item-preview">{n.content ? n.content.slice(0, 40) : '无附加文字'}</div>
                <Text type="secondary" className="note-item-time">
                  {dayjs(n.updated_at).format('YYYY-MM-DD HH:mm')}
                </Text>
              </div>
            </List.Item>
          )}
        />
      </Sider>
      <Content className="editor-pane">
        {active ? (
          <>
            <div className="editor-toolbar">
              {saving ? <Tag color="processing">保存中…</Tag> : <Tag color="success">已保存</Tag>}
              <Popconfirm title="删除这条记录？" okText="删除" cancelText="取消" onConfirm={() => removeNote(active.id)}>
                <Button danger type="text" icon={<DeleteOutlined />} />
              </Popconfirm>
            </div>
            <Input
              className="editor-title"
              placeholder="标题"
              value={active.title}
              onChange={(e) => onEdit({ title: e.target.value })}
              variant="borderless"
            />
            <div className="editor-meta">更新于 {dayjs(active.updated_at).format('YYYY-MM-DD HH:mm:ss')}</div>
            <Input.TextArea
              className="editor-content"
              placeholder="开始记录…"
              value={active.content}
              onChange={(e) => onEdit({ content: e.target.value })}
            />
          </>
        ) : (
          <Empty description="选择一条记录，或点击「新建」开始" style={{ marginTop: 120 }} />
        )}
      </Content>
    </Layout>
  )
}
