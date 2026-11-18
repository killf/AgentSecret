import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@ant-design/v5-patch-for-react-19';
import { App as AntApp, ConfigProvider, theme as antdTheme } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import App from './App';
import './styles.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ConfigProvider
      locale={zhCN}
      theme={{
        algorithm: antdTheme.defaultAlgorithm,
        token: {
          colorPrimary: '#635BFF',
          colorInfo: '#635BFF',
          colorLink: '#635BFF',
          colorBgBase: '#FFFFFF',
          colorBgContainer: '#FFFFFF',
          colorBgElevated: '#FFFFFF',
          colorBgLayout: '#F6F7F9',
          colorBorder: '#DCE1E8',
          colorBorderSecondary: '#E7EBF0',
          colorText: '#1A1F36',
          colorTextSecondary: '#5C6675',
          colorTextTertiary: '#798394',
          borderRadius: 7,
          controlHeight: 32,
          fontFamily: 'Inter, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif',
          fontSize: 13,
          boxShadow: '0 1px 2px rgba(50, 50, 93, .08)',
        },
        components: {
          Button: { fontWeight: 600, primaryShadow: 'none', defaultShadow: 'none', paddingInlineSM: 10 },
          Card: { paddingLG: 18, headerFontSize: 13.5, headerHeight: 44 },
          Layout: { headerBg: '#FFFFFF', bodyBg: '#F6F7F9', siderBg: '#FFFFFF' },
          Menu: { itemHeight: 32, itemMarginInline: 8, iconSize: 15, itemSelectedBg: '#F0EFFE', itemSelectedColor: '#3B3499', itemColor: '#5F6875', itemHoverBg: '#F3F5F7' },
          Table: { headerBg: '#FAFBFC', headerColor: '#687383', rowHoverBg: '#F9FAFB', borderColor: '#EDF0F3', cellPaddingBlockSM: 8, cellPaddingInlineSM: 12 },
          Tabs: { itemColor: '#5F6875', itemSelectedColor: '#1A1F36', inkBarColor: '#635BFF' },
          Drawer: { colorBgElevated: '#FFFFFF' },
          Form: { itemMarginBottom: 14 },
        },
      }}
    >
      <AntApp>
        <App />
      </AntApp>
    </ConfigProvider>
  </StrictMode>,
);