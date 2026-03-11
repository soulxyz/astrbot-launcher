import { useEffect, useState } from 'react';
import { Button, Space, Typography } from 'antd';
import { getVersion } from '@tauri-apps/api/app';
import { PageHeader } from '../components';
import { useUpdateStore } from '../stores';
import { message } from '../antdStatic';

const { Text, Title } = Typography;

export default function About() {
  const [version, setVersion] = useState('');
  const { hasUpdate, newVersion, checking, installing, checkForUpdate, installUpdate } =
    useUpdateStore();

  useEffect(() => {
    void getVersion().then(setVersion);
  }, []);

  const handleCheckUpdate = async () => {
    await checkForUpdate();
    const state = useUpdateStore.getState();
    if (!state.hasUpdate) {
      message.success('已是最新版本');
    }
  };

  return (
    <>
      <PageHeader title="关于" />
      <div style={{ display: 'flex', justifyContent: 'center', paddingTop: 48 }}>
        <Space direction="vertical" align="center" size="large">
          <img src="/logo.png" alt="AstrBot Launcher" width={96} height={96} />
          <Title level={4} style={{ margin: 0 }}>
            AstrBot Launcher
          </Title>
          <Text type="secondary">v{version}</Text>

          <Button
            type={hasUpdate ? 'primary' : 'default'}
            loading={hasUpdate ? installing : checking}
            onClick={hasUpdate ? installUpdate : handleCheckUpdate}
          >
            {hasUpdate ? `更新到 v${newVersion}` : '检查更新'}
          </Button>
        </Space>
      </div>
    </>
  );
}
