import React, { useRef } from 'react';
import { Play, Square, Save, Upload, Download } from 'lucide-react';

interface ToolbarProps {
    onRun: () => void;
    onStop: () => void;
    onSave: () => void;
    onExport: () => void;
    onImport: (file: File) => void;
    isRunning: boolean;
    workflowName: string;
    onNameChange: (name: string) => void;
}

export const Toolbar: React.FC<ToolbarProps> = ({
    onRun,
    onStop,
    onSave,
    onExport,
    onImport,
    isRunning,
    workflowName,
    onNameChange
}) => {
    const fileInputRef = useRef<HTMLInputElement>(null);

    const handleImportClick = () => {
        fileInputRef.current?.click();
    };

    const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0];
        if (file) {
            onImport(file);
        }
        // Reset input so same file can be selected again
        if (event.target) {
            event.target.value = '';
        }
    };
    return (
        <div className="toolbar">
            <div className="toolbar-left">
                <div className="logo">RustFlow</div>
                <input
                    type="text"
                    className="workflow-name-input"
                    value={workflowName}
                    onChange={(e) => onNameChange(e.target.value)}
                    placeholder="Untitled Workflow"
                />
            </div>

            <div className="toolbar-right">
                <button
                    className="toolbar-button save"
                    onClick={onSave}
                    title="Save Workflow"
                >
                    <Save size={18} />
                    <span className="ml-2">Save</span>
                </button>

                <button
                    className="toolbar-button export"
                    onClick={onExport}
                    title="Export Workflow (YAML)"
                >
                    <Download size={18} />
                    <span className="ml-2">Export</span>
                </button>

                <button
                    className="toolbar-button import"
                    onClick={handleImportClick}
                    title="Import Workflow (YAML)"
                >
                    <Upload size={18} />
                    <span className="ml-2">Import</span>
                </button>
                <input
                    type="file"
                    ref={fileInputRef}
                    onChange={handleFileChange}
                    accept=".yaml,.yml,.json"
                    style={{ display: 'none' }}
                />

                <div className="divider"></div>

                <button
                    className={`toolbar-button run ${isRunning ? 'disabled' : ''}`}
                    onClick={onRun}
                    disabled={isRunning}
                    title="Run Workflow"
                >
                    <Play size={18} fill={isRunning ? "#9ca3af" : "currentColor"} />
                    <span className="ml-2">Run</span>
                </button>

                <button
                    className={`toolbar-button stop ${!isRunning ? 'disabled' : ''}`}
                    onClick={onStop}
                    disabled={!isRunning}
                    title="Stop Workflow"
                >
                    <Square size={18} fill={!isRunning ? "#9ca3af" : "currentColor"} />
                    <span className="ml-2">Stop</span>
                </button>
            </div>
        </div>
    );
};
