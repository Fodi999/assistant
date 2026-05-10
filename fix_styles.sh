sed -i '' '/\.prop-section {/ i\
    .prop-vector {\
      display: flex;\
      flex-direction: column;\
      gap: 4px;\
      margin-bottom: 12px;\
    }\
    .prop-row {\
      display: flex;\
      align-items: center;\
      background: rgba(15, 23, 42, 0.6);\
      border: 1px solid rgba(255, 255, 255, 0.1);\
      border-radius: 4px;\
      overflow: hidden;\
    }\
    .prop-row-label {\
      min-width: 20px;\
      text-align: center;\
      font-size: 11px;\
      font-weight: bold;\
      background: rgba(255, 255, 255, 0.05);\
      padding: 6px 0;\
      border-right: 1px solid rgba(255, 255, 255, 0.1);\
    }\
    .prop-row-label.x { color: #f87171; border-left: 2px solid #f87171; }\
    .prop-row-label.y { color: #4ade80; border-left: 2px solid #4ade80; }\
    .prop-row-label.z { color: #60a5fa; border-left: 2px solid #60a5fa; }\
    .prop-row input {\
      flex: 1;\
      background: none;\
      border: none;\
      color: #e2e8f0;\
      font-family: monospace;\
      font-size: 12px;\
      padding: 6px 8px;\
      width: 100%;\
      outline: none;\
    }\
    .prop-row input:focus {\
      background: rgba(255, 255, 255, 0.05);\
    }\
' src/web/home/matter_lab_styles.rs
