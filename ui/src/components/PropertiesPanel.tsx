import React from 'react';
import { type Node } from '@xyflow/react';
import { type NodeType, type NodeProperty } from '../types';

interface PropertiesPanelProps {
    selectedNode: Node | null;
    nodeTypes: NodeType[];
    onUpdateNodeData: (id: string, data: any) => void;
}

export const PropertiesPanel: React.FC<PropertiesPanelProps> = ({ selectedNode, nodeTypes, onUpdateNodeData }) => {
    if (!selectedNode) {
        return (
            <div className="properties-panel">
                <div className="properties-header">Properties</div>
                <div className="properties-content">
                    <p className="text-gray text-sm">Select a node to edit its properties.</p>
                </div>
            </div>
        );
    }

    const nodeTypeId = selectedNode.data.nodeType as string;
    const nodeType = nodeTypes.find(t => t.id === nodeTypeId);

    if (!nodeType) {
        return (
            <div className="properties-panel">
                <div className="properties-header">Unknown Node</div>
                <div className="properties-content">
                    <p>Node type ID: {nodeTypeId}</p>
                </div>
            </div>
        );
    }

    const handleChange = (key: string, value: any) => {
        onUpdateNodeData(selectedNode.id, {
            ...selectedNode.data,
            [key]: value,
        });
    };

    return (
        <div className="properties-panel">
            <div className="properties-header">
                {nodeType.label} Settings
            </div>
            <div className="properties-content">
                <div className="form-group">
                    <label className="form-label">Label</label>
                    <input
                        type="text"
                        className="form-input"
                        value={selectedNode.data.label as string || nodeType.label}
                        onChange={(e) => handleChange('label', e.target.value)}
                    />
                </div>

                {nodeType.description && (
                    <div className="mb-4 text-xs text-gray">
                        {nodeType.description}
                    </div>
                )}

                <hr className="my-4 border-gray" />

                {nodeType.properties.map((prop: NodeProperty) => (
                    <div key={prop.name} className="form-group">
                        <label className="form-label">
                            {prop.label}
                            {prop.required && <span className="text-red ml-1">*</span>}
                        </label>

                        {renderInput(prop, selectedNode.data[prop.name], (val) => handleChange(prop.name, val))}
                    </div>
                ))}
            </div>
        </div>
    );
};

function renderInput(prop: NodeProperty, value: any, onChange: (val: any) => void) {
    const val = value !== undefined ? value : (prop.default || '');

    switch (prop.type) {
        case 'select':
            return (
                <select
                    className="form-select"
                    value={val}
                    onChange={(e) => onChange(e.target.value)}
                >
                    {prop.options?.map(opt => (
                        <option key={opt} value={opt}>{opt}</option>
                    ))}
                </select>
            );
        case 'boolean':
            return (
                <input
                    type="checkbox"
                    checked={!!val}
                    onChange={(e) => onChange(e.target.checked)}
                />
            );
        case 'json':
        case 'code':
            return (
                <textarea
                    className="form-textarea"
                    value={val}
                    onChange={(e) => onChange(e.target.value)}
                    placeholder={prop.type === 'json' ? '{}' : '// code'}
                />
            );
        case 'number':
            return (
                <input
                    type="number"
                    className="form-input"
                    value={val}
                    onChange={(e) => onChange(Number(e.target.value))}
                />
            );
        default: // text
            return (
                <input
                    type="text"
                    className="form-input"
                    value={val}
                    onChange={(e) => onChange(e.target.value)}
                />
            );
    }
}
