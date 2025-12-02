import { useState, useEffect, useCallback } from 'react';
import yaml from 'js-yaml';
import { ReactFlowProvider, useNodesState, useEdgesState, type Node, type Edge } from '@xyflow/react';
import { Sidebar } from './components/Sidebar';
import { FlowCanvas } from './components/FlowCanvas';
import { PropertiesPanel } from './components/PropertiesPanel';
import { Toolbar } from './components/Toolbar';
import { fetchNodeTypes, runWorkflow, getJobStatus } from './api';
import { type NodeType, type ExecutionEvent } from './types';
import { useWebSocket } from './hooks/useWebSocket';
import { DebugPanel } from './components/DebugPanel';
import './layout.css';

function App() {
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [nodeTypes, setNodeTypes] = useState<NodeType[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [workflowName, setWorkflowName] = useState('');
  const [events, setEvents] = useState<ExecutionEvent[]>([]);
  const [edgeActivity, setEdgeActivity] = useState<Record<string, number>>({});
  const [nodeStatus, setNodeStatus] = useState<Record<string, string>>({});

  const { connect, lastEvent } = useWebSocket();

  useEffect(() => {
    if (lastEvent) {
      setEvents(prev => [...prev, lastEvent]);

      if (lastEvent.type === 'EdgeData' && lastEvent.from && lastEvent.to) {
        // Create a key for the edge: "source-target"
        const edgeKey = `${lastEvent.from}-${lastEvent.to}`;
        setEdgeActivity(prev => ({ ...prev, [edgeKey]: Date.now() }));
      } else if (lastEvent.type === 'NodeStart' && lastEvent.node_id) {
        setNodeStatus(prev => ({ ...prev, [lastEvent.node_id!]: 'running' }));
      } else if (lastEvent.type === 'NodeFinish' && lastEvent.node_id) {
        setNodeStatus(prev => ({ ...prev, [lastEvent.node_id!]: 'completed' }));
      } else if (lastEvent.type === 'NodeError' && lastEvent.node_id) {
        setNodeStatus(prev => ({ ...prev, [lastEvent.node_id!]: 'failed' }));
      }
    }
  }, [lastEvent]);

  useEffect(() => {
    fetchNodeTypes().then(setNodeTypes).catch(console.error);
  }, []);

  const onNodeSelect = (node: Node | null) => {
    setSelectedNode(node);
  };

  const onUpdateNodeData = (id: string, newData: any) => {
    setNodes((nds) =>
      nds.map((node) => {
        if (node.id === id) {
          // Update selected node as well if it's the one being updated
          if (selectedNode && selectedNode.id === id) {
            setSelectedNode({ ...node, data: newData });
          }
          return { ...node, data: newData };
        }
        return node;
      })
    );
  };

  const handleRun = async () => {
    setIsRunning(true);
    try {
      // Convert React Flow graph to Backend Workflow Definition
      const workflow = {
        nodes: nodes.map(node => ({
          id: node.id,
          type: node.data.nodeType || node.type, // Use specific ID from data if available (e.g. manual_trigger), else type
          config: node.data // Pass all data as config
        })),
        edges: edges.map(edge => ({
          from: edge.source,
          from_port: 0, // Default to 0 for now
          to: edge.target,
          to_port: 0 // Default to 0 for now
        }))
      };

      console.log('Running workflow:', workflow);
      const result = await runWorkflow(workflow);
      console.log('Execution initiated:', result);

      if (result.job_id) {
        setEvents([]); // Clear previous logs
        setNodeStatus({}); // Clear previous status
        connect(result.job_id); // Connect WS
        pollJob(result.job_id);
      } else {
        alert('Workflow executed successfully (sync)!');
        setIsRunning(false);
      }
    } catch (error) {
      console.error('Execution failed:', error);
      alert('Workflow execution failed. Check console for details.');
      setIsRunning(false);
    }
  };

  const pollJob = async (id: string) => {
    const interval = setInterval(async () => {
      try {
        const status = await getJobStatus(id);
        console.log('Job status:', status);

        if (status.status === 'completed') {
          clearInterval(interval);
          setIsRunning(false);
        } else if (status.status === 'failed') {
          clearInterval(interval);
          setIsRunning(false);
        }
      } catch (e) {
        clearInterval(interval);
        setIsRunning(false);
        console.error('Polling failed:', e);
      }
    }, 1000);
  };

  const handleStop = () => {
    // TODO: Implement stop functionality when backend supports it
    setIsRunning(false);
  };

  const handleSave = () => {
    console.log('Saving workflow:', workflowName);
    alert(`Workflow "${workflowName || 'Untitled'}" saved! (Mock)`);
  };

  const handleExport = useCallback(() => {
    const workflow = {
      name: workflowName,
      nodes: nodes.map(node => ({
        id: node.id,
        type: node.data.nodeType || node.type,
        config: node.data,
        position: node.position // Save UI position
      })),
      edges: edges.map(edge => ({
        from: edge.source,
        to: edge.target,
        id: edge.id
      }))
    };

    const yamlStr = yaml.dump(workflow);
    const blob = new Blob([yamlStr], { type: 'text/yaml' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${workflowName || 'workflow'}.yaml`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [nodes, edges, workflowName]);

  const handleImport = useCallback((file: File) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const content = e.target?.result as string;
        const workflow: any = yaml.load(content);

        if (workflow.name) setWorkflowName(workflow.name);

        if (Array.isArray(workflow.nodes)) {
          const newNodes: Node[] = workflow.nodes.map((n: any) => ({
            id: n.id,
            type: 'workflowNode', // Always use our custom node type wrapper
            position: n.position || { x: 0, y: 0 },
            data: {
              ...n.config,
              nodeType: n.type // Restore original type
            }
          }));
          setNodes(newNodes);
        }

        if (Array.isArray(workflow.edges)) {
          const newEdges: Edge[] = workflow.edges.map((e: any) => ({
            id: e.id || `e${e.from}-${e.to}`,
            source: e.from,
            target: e.to,
            type: 'default',
            animated: false
          }));
          setEdges(newEdges);
        }
      } catch (error) {
        console.error('Failed to import workflow:', error);
        alert('Failed to import workflow. Invalid YAML?');
      }
    };
    reader.readAsText(file);
  }, [setNodes, setEdges]);

  return (
    <ReactFlowProvider>
      <div className="app-layout">
        <Toolbar
          onRun={handleRun}
          onStop={handleStop}
          onSave={handleSave}
          onExport={handleExport}
          onImport={handleImport}
          isRunning={isRunning}
          workflowName={workflowName}
          onNameChange={setWorkflowName}
        />
        <div className="main-content">
          <Sidebar />
          <FlowCanvas
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            setNodes={setNodes}
            setEdges={setEdges}
            onNodeSelect={onNodeSelect}
            edgeActivity={edgeActivity}
            nodeStatus={nodeStatus}
          />
          <PropertiesPanel
            selectedNode={selectedNode}
            nodeTypes={nodeTypes}
            onUpdateNodeData={onUpdateNodeData}
          />
          <DebugPanel events={events} />
        </div>
      </div>
    </ReactFlowProvider>
  );
}

export default App;
