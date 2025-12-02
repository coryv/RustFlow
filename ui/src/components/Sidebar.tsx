import React, { useEffect, useState } from 'react';
import { type NodeType } from '../types';
import { fetchNodeTypes } from '../api';
import {
    Zap,
    Activity,
    GitBranch,
    Database,
    Bot,
    Cpu,
    FileText,
    ChevronDown,
    ChevronRight
} from 'lucide-react';

const categoryIcons: Record<string, React.ReactNode> = {
    'Trigger': <Zap size={16} className="mr-2" />,
    'Action': <Activity size={16} className="mr-2" />,
    'Logic': <GitBranch size={16} className="mr-2" />,
    'Integration': <Database size={16} className="mr-2" />,
    'AI': <Bot size={16} className="mr-2" />,
    'Models': <Cpu size={16} className="mr-2" />,
    'Data Processing': <FileText size={16} className="mr-2" />,
};

// Mapping from backend category to UI section
const getSectionForCategory = (category: string): 'Triggers' | 'Actions' => {
    if (category === 'Trigger') return 'Triggers';
    return 'Actions';
};

// Mapping for sub-groups
const getSubGroupForCategory = (category: string): string => {
    if (category === 'Trigger') return 'Core Triggers';
    if (category === 'Action') return 'Core Actions';
    return category; // Logic, Integration, AI, Models, Data Processing
};

export const Sidebar: React.FC = () => {
    const [nodeTypes, setNodeTypes] = useState<NodeType[]>([]);
    const [loading, setLoading] = useState(true);
    const [expandedSections, setExpandedSections] = useState<Record<string, boolean>>({
        'Triggers': true,
        'Actions': true,
    });
    const [expandedSubGroups, setExpandedSubGroups] = useState<Record<string, boolean>>({
        'Core Triggers': true,
        'Core Actions': true,
        'AI': true,
    });

    useEffect(() => {
        fetchNodeTypes().then(types => {
            setNodeTypes(types);
            setLoading(false);
        }).catch(err => {
            console.error(err);
            setLoading(false);
        });
    }, []);

    const onDragStart = (event: React.DragEvent, nodeType: NodeType) => {
        event.dataTransfer.setData('application/reactflow', JSON.stringify(nodeType));
        event.dataTransfer.effectAllowed = 'move';
    };

    const toggleSection = (section: string) => {
        setExpandedSections(prev => ({ ...prev, [section]: !prev[section] }));
    };

    const toggleSubGroup = (group: string) => {
        setExpandedSubGroups(prev => ({ ...prev, [group]: !prev[group] }));
    };

    // Group nodes by Section -> SubGroup
    const structuredNodes = nodeTypes.reduce((acc, node) => {
        const section = getSectionForCategory(node.category);
        const subGroup = getSubGroupForCategory(node.category);

        if (!acc[section]) acc[section] = {};
        if (!acc[section][subGroup]) acc[section][subGroup] = [];

        acc[section][subGroup].push(node);
        return acc;
    }, {} as Record<string, Record<string, NodeType[]>>);

    if (loading) return <div className="sidebar">Loading...</div>;

    return (
        <div className="sidebar">
            {['Triggers', 'Actions'].map(section => (
                <div key={section} className="sidebar-section">
                    <div
                        className="section-header"
                        onClick={() => toggleSection(section)}
                    >
                        {expandedSections[section] ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                        <span className="ml-1">{section}</span>
                    </div>

                    {expandedSections[section] && structuredNodes[section] && (
                        <div className="section-content">
                            {Object.entries(structuredNodes[section]).map(([subGroup, nodes]) => (
                                <div key={subGroup} className="sub-group">
                                    <div
                                        className="sub-group-header"
                                        onClick={() => toggleSubGroup(subGroup)}
                                    >
                                        {expandedSubGroups[subGroup] ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                                        <span className="ml-1">{subGroup}</span>
                                    </div>

                                    {expandedSubGroups[subGroup] && (
                                        <div className="sub-group-content">
                                            {nodes.map((node) => (
                                                <div
                                                    key={node.id}
                                                    className="node-item"
                                                    onDragStart={(event) => onDragStart(event, node)}
                                                    draggable
                                                >
                                                    {categoryIcons[node.category] || <Activity size={16} className="mr-2" />}
                                                    {node.label}
                                                </div>
                                            ))}
                                        </div>
                                    )}
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            ))}
        </div>
    );
};
