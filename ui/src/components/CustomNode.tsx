import React, { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import {
    Zap,
    Activity,
    GitBranch,
    Database,
    Bot,
    Cpu,
    FileText
} from 'lucide-react';

const categoryIcons: Record<string, React.ReactNode> = {
    'Trigger': <Zap size={16} />,
    'Action': <Activity size={16} />,
    'Logic': <GitBranch size={16} />,
    'Integration': <Database size={16} />,
    'AI': <Bot size={16} />,
    'Models': <Cpu size={16} />,
    'Data Processing': <FileText size={16} />,
};

export const CustomNode = memo(({ data, selected }: NodeProps) => {
    const category = data.category as string;
    const label = data.label as string;
    const isTrigger = category === 'Trigger';
    const isModel = category === 'Models';

    const getCategoryClass = (cat: string) => {
        switch (cat) {
            case 'Trigger': return 'bg-yellow';
            case 'AI': return 'bg-purple';
            case 'Models': return 'bg-green';
            default: return 'bg-blue';
        }
    };

    return (
        <div className={`custom-node ${selected ? 'selected' : ''}`}>
            {!isTrigger && !isModel && (
                <Handle type="target" position={Position.Left} className="handle-target" />
            )}

            {isModel && (
                <Handle type="source" position={Position.Right} className="handle-model" />
            )}

            <div className={`node-icon ${getCategoryClass(category)}`}>
                {categoryIcons[category] || <Activity size={16} />}
            </div>
            <div className="node-content">
                <div className="node-label">{label}</div>
                <div className="node-category-label">{category}</div>
            </div>

            {!isModel && (
                <Handle type="source" position={Position.Right} className="handle-source" />
            )}
        </div>
    );
});
